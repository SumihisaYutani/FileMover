use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};
use filemover_types::{
    MovePlan, PlanNode, PlanNodeId, PlanSummary, OpKind, FolderHit, 
    Rule, PlanOptions, FileMoverError, ConflictPolicy
};
use crate::template::TemplateEngine;
use crate::conflict_resolver::ConflictResolver;
use crate::validator::PlanValidator;

pub struct MovePlanner {
    template_engine: TemplateEngine,
    conflict_resolver: ConflictResolver,
    validator: PlanValidator,
}

impl MovePlanner {
    pub fn new() -> Self {
        Self {
            template_engine: TemplateEngine::new(),
            conflict_resolver: ConflictResolver::new(),
            validator: PlanValidator::new(),
        }
    }

    pub fn create_plan(
        &mut self,
        folder_hits: &[FolderHit],
        rules: &[Rule],
        options: PlanOptions,
    ) -> Result<MovePlan, FileMoverError> {
        info!("Creating move plan for {} folder hits", folder_hits.len());

        let mut nodes = HashMap::new();
        let mut roots = Vec::new();
        let mut rule_map: HashMap<uuid::Uuid, &Rule> = HashMap::new();

        // ルールマップを作成
        for rule in rules {
            rule_map.insert(rule.id, rule);
        }

        // 各フォルダヒットからプランノードを生成
        for hit in folder_hits {
            let node_id = PlanNodeId::new();
            
            let plan_node = self.create_plan_node(hit, &rule_map, &options, node_id)?;
            
            nodes.insert(node_id, plan_node);
            roots.push(node_id);
        }

        // 衝突解決とバリデーション
        self.resolve_conflicts_and_validate(&mut nodes, &options)?;

        // サマリを計算
        let summary = self.calculate_summary(&nodes);

        let mut plan = MovePlan {
            roots,
            nodes,
            summary,
        };

        // 最終バリデーション
        let validation_result = self.validator.validate_full_plan(&plan)?;
        self.apply_validation_result(&mut plan, validation_result)?;

        info!("Move plan created successfully with {} nodes", plan.nodes.len());
        Ok(plan)
    }

    fn create_plan_node(
        &mut self,
        hit: &FolderHit,
        rule_map: &HashMap<uuid::Uuid, &Rule>,
        options: &PlanOptions,
        node_id: PlanNodeId,
    ) -> Result<PlanNode, FileMoverError> {
        let rule = hit.matched_rule
            .and_then(|rule_id| rule_map.get(&rule_id))
            .ok_or_else(|| FileMoverError::PlanValidation {
                message: format!("Rule not found for folder: {}", hit.path.display()),
            })?;

        // テンプレートを展開して移動先パスを生成
        let dest_path = self.template_engine.expand_template(rule, &hit.path)?;
        
        // 操作種別を決定
        let op_kind = self.determine_operation_kind(&hit.path, &dest_path);

        let plan_node = PlanNode {
            id: node_id,
            is_dir: true, // フォルダのみを扱う
            name_before: hit.name.clone(),
            path_before: hit.path.clone(),
            name_after: dest_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&hit.name)
                .to_string(),
            path_after: dest_path,
            kind: op_kind,
            size_bytes: hit.size_bytes,
            warnings: hit.warnings.clone(),
            conflicts: Vec::new(), // 後で衝突解決で設定
            children: Vec::new(),   // 単純な実装ではフラット構造
            rule_id: hit.matched_rule,
        };

        Ok(plan_node)
    }

    fn determine_operation_kind(&self, source: &PathBuf, dest: &PathBuf) -> OpKind {
        if source == dest {
            return OpKind::None;
        }

        // ボリュームが異なる場合はCopy+Delete
        if self.is_cross_volume(source, dest) {
            OpKind::CopyDelete
        } else {
            OpKind::Move
        }
    }

    fn is_cross_volume(&self, source: &PathBuf, dest: &PathBuf) -> bool {
        let source_drive = source.components().next();
        let dest_drive = dest.components().next();
        source_drive != dest_drive
    }

    fn resolve_conflicts_and_validate(
        &mut self,
        nodes: &mut HashMap<PlanNodeId, PlanNode>,
        options: &PlanOptions,
    ) -> Result<(), FileMoverError> {
        debug!("Resolving conflicts for {} nodes", nodes.len());

        for (node_id, node) in nodes.iter_mut() {
            if matches!(node.kind, OpKind::Skip | OpKind::None) {
                continue;
            }

            // 衝突解決
            let policy = options.default_conflict_policy;
            let (resolved_path, conflicts) = self.conflict_resolver
                .resolve_conflicts(&node.path_after, policy)?;

            // 解決されたパスを反映
            if resolved_path != node.path_after {
                node.path_after = resolved_path;
                node.name_after = node.path_after
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&node.name_before)
                    .to_string();
            }

            // 衝突を記録
            node.conflicts = conflicts;

            debug!("Node {}: {} conflicts resolved", node_id.0, node.conflicts.len());
        }

        Ok(())
    }

    fn calculate_summary(&self, nodes: &HashMap<PlanNodeId, PlanNode>) -> PlanSummary {
        let mut summary = PlanSummary::default();

        for node in nodes.values() {
            if matches!(node.kind, OpKind::Skip | OpKind::None) {
                continue;
            }

            if node.is_dir {
                summary.count_dirs += 1;
            } else {
                summary.count_files += 1;
            }

            if let Some(size) = node.size_bytes {
                summary.total_bytes = Some(summary.total_bytes.unwrap_or(0) + size);
            }

            summary.conflicts += node.conflicts.len() as u64;
            summary.warnings += node.warnings.len() as u64;

            if self.is_cross_volume(&node.path_before, &node.path_after) {
                summary.cross_volume += 1;
            }
        }

        debug!("Plan summary: {} dirs, {} conflicts, {} warnings", 
               summary.count_dirs, summary.conflicts, summary.warnings);

        summary
    }

    fn apply_validation_result(
        &mut self,
        plan: &mut MovePlan,
        validation_result: filemover_types::ValidationDelta,
    ) -> Result<(), FileMoverError> {
        // バリデーション結果を適用
        for &node_id in &validation_result.affected_nodes {
            if let Some(node) = plan.nodes.get_mut(&node_id) {
                // 新しい衝突を追加
                for conflict in &validation_result.new_conflicts {
                    if !node.conflicts.contains(conflict) {
                        node.conflicts.push(conflict.clone());
                    }
                }
            }
        }

        // サマリを更新
        self.apply_summary_diff(&mut plan.summary, &validation_result.summary_diff);

        Ok(())
    }

    fn apply_summary_diff(&self, summary: &mut PlanSummary, diff: &filemover_types::PlanSummaryDiff) {
        summary.count_dirs = (summary.count_dirs as i64 + diff.count_dirs_delta).max(0) as u64;
        summary.count_files = (summary.count_files as i64 + diff.count_files_delta).max(0) as u64;
        summary.cross_volume = (summary.cross_volume as i64 + diff.cross_volume_delta).max(0) as u64;
        summary.conflicts = (summary.conflicts as i64 + diff.conflicts_delta).max(0) as u64;
        summary.warnings = (summary.warnings as i64 + diff.warnings_delta).max(0) as u64;

        if let Some(bytes_delta) = diff.total_bytes_delta {
            summary.total_bytes = summary.total_bytes
                .map(|current| (current as i64 + bytes_delta).max(0) as u64);
        }
    }

    pub fn simulate_plan(&self, plan: &MovePlan) -> Result<SimulationReport, FileMoverError> {
        let mut report = SimulationReport::new();

        for node in plan.nodes.values() {
            match node.kind {
                OpKind::Skip | OpKind::None => {
                    report.skipped_count += 1;
                }
                _ => {
                    if node.conflicts.is_empty() {
                        report.success_estimate += 1;
                    } else {
                        report.conflicts_remaining += 1;
                    }
                }
            }
        }

        // 推定実行時間（簡易計算）
        let estimated_seconds = plan.summary.count_dirs * 2; // フォルダあたり2秒と仮定
        report.estimated_duration_secs = estimated_seconds;

        Ok(report)
    }

    pub fn update_plan_with_change(
        &mut self,
        plan: &mut MovePlan,
        change: filemover_types::NodeChange,
    ) -> Result<filemover_types::ValidationDelta, FileMoverError> {
        let validation_result = self.validator.validate_incremental_change(plan, change)?;
        self.apply_validation_result(plan, validation_result.clone())?;
        Ok(validation_result)
    }
}

impl Default for MovePlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulationReport {
    pub success_estimate: u64,
    pub conflicts_remaining: u64,
    pub skipped_count: u64,
    pub estimated_duration_secs: u64,
}

impl SimulationReport {
    pub fn new() -> Self {
        Self {
            success_estimate: 0,
            conflicts_remaining: 0,
            skipped_count: 0,
            estimated_duration_secs: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filemover_types::{PatternSpec, Warning};
    use std::path::PathBuf;

    fn create_test_rule() -> Rule {
        Rule::new(
            PatternSpec::new_glob("test*"),
            PathBuf::from("D:\\Archive"),
            "{yyyy}\\{name}".to_string(),
        )
    }

    fn create_test_folder_hit() -> FolderHit {
        FolderHit {
            path: PathBuf::from("C:\\Source\\test_folder"),
            name: "test_folder".to_string(),
            matched_rule: None,
            dest_preview: None,
            warnings: Vec::new(),
            size_bytes: Some(1024 * 1024), // 1MB
        }
    }

    #[test]
    fn test_plan_creation() {
        let mut planner = MovePlanner::new();
        let rule = create_test_rule();
        let mut hit = create_test_folder_hit();
        hit.matched_rule = Some(rule.id);

        let rules = vec![rule];
        let hits = vec![hit];
        let options = PlanOptions::default();

        let plan = planner.create_plan(&hits, &rules, options).unwrap();
        
        assert_eq!(plan.roots.len(), 1);
        assert_eq!(plan.nodes.len(), 1);
        assert_eq!(plan.summary.count_dirs, 1);
    }

    #[test]
    fn test_cross_volume_detection() {
        let planner = MovePlanner::new();
        
        let source = PathBuf::from("C:\\Source");
        let dest_same = PathBuf::from("C:\\Dest");
        let dest_different = PathBuf::from("D:\\Dest");
        
        assert!(!planner.is_cross_volume(&source, &dest_same));
        assert!(planner.is_cross_volume(&source, &dest_different));
    }

    #[test]
    fn test_operation_kind_determination() {
        let planner = MovePlanner::new();
        
        let source = PathBuf::from("C:\\Source");
        let dest_same_volume = PathBuf::from("C:\\Dest");
        let dest_different_volume = PathBuf::from("D:\\Dest");
        let dest_same_path = source.clone();
        
        assert_eq!(planner.determine_operation_kind(&source, &dest_same_volume), OpKind::Move);
        assert_eq!(planner.determine_operation_kind(&source, &dest_different_volume), OpKind::CopyDelete);
        assert_eq!(planner.determine_operation_kind(&source, &dest_same_path), OpKind::None);
    }

    #[test]
    fn test_simulation_report() {
        let mut planner = MovePlanner::new();
        let rule = create_test_rule();
        let mut hit = create_test_folder_hit();
        hit.matched_rule = Some(rule.id);

        let rules = vec![rule];
        let hits = vec![hit];
        let options = PlanOptions::default();

        let plan = planner.create_plan(&hits, &rules, options).unwrap();
        let report = planner.simulate_plan(&plan).unwrap();
        
        assert_eq!(report.success_estimate + report.conflicts_remaining + report.skipped_count, 1);
    }
}