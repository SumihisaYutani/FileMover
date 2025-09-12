use std::collections::{HashMap, HashSet};
use filemover_types::{
    MovePlan, PlanNode, PlanNodeId, ValidationDelta, NodeChange, 
    Conflict, Warning, OpKind, PlanSummary, PlanSummaryDiff, FileMoverError
};
use tracing::{debug, warn};
use crate::conflict_resolver::ConflictResolver;

pub struct PlanValidator {
    conflict_resolver: ConflictResolver,
}

impl PlanValidator {
    pub fn new() -> Self {
        Self {
            conflict_resolver: ConflictResolver::new(),
        }
    }

    pub fn validate_full_plan(&mut self, plan: &MovePlan) -> Result<ValidationDelta, FileMoverError> {
        debug!("Starting full plan validation for {} nodes", plan.nodes.len());
        
        let mut new_conflicts = Vec::new();
        let mut affected_nodes = Vec::new();
        
        // 全ノードを検証
        for (node_id, node) in &plan.nodes {
            let node_conflicts = self.validate_single_node(node, plan)?;
            
            if !node_conflicts.is_empty() {
                new_conflicts.extend(node_conflicts);
                affected_nodes.push(*node_id);
            }
        }

        // サマリの再計算
        let summary_diff = self.recalculate_summary(plan)?;

        Ok(ValidationDelta {
            affected_nodes,
            new_conflicts,
            resolved_conflicts: Vec::new(), // 全体検証では解決された衝突は追跡しない
            summary_diff,
        })
    }

    pub fn validate_incremental_change(
        &mut self,
        plan: &mut MovePlan,
        change: NodeChange,
    ) -> Result<ValidationDelta, FileMoverError> {
        debug!("Processing incremental change: {:?}", change);
        
        match change {
            NodeChange::SetSkip(node_id, skip) => {
                self.handle_skip_change(plan, node_id, skip)
            }
            NodeChange::SetConflictPolicy(node_id, policy) => {
                self.handle_policy_change(plan, node_id, policy)
            }
            NodeChange::RenameNode(node_id, new_name) => {
                self.handle_rename_change(plan, node_id, new_name)
            }
            NodeChange::ExcludeNode(node_id) => {
                self.handle_exclude_change(plan, node_id)
            }
        }
    }

    fn validate_single_node(&mut self, node: &PlanNode, plan: &MovePlan) -> Result<Vec<Conflict>, FileMoverError> {
        let mut conflicts = Vec::new();

        // スキップされたノードは検証しない
        if matches!(node.kind, OpKind::Skip | OpKind::None) {
            return Ok(conflicts);
        }

        // パス関連の検証
        if node.path_before == node.path_after {
            // 移動先が移動元と同じ場合は何もしない
            return Ok(conflicts);
        }

        // 循環参照の検出
        if self.detect_cycle(node, plan)? {
            conflicts.push(Conflict::CycleDetected);
        }

        // 移動先が移動元の子ディレクトリかチェック
        if node.path_after.starts_with(&node.path_before) {
            conflicts.push(Conflict::DestInsideSource);
        }

        // ディスク容量チェック
        if let Some(size) = node.size_bytes {
            if let Some(space_conflict) = self.check_disk_space(&node.path_after, size)? {
                conflicts.push(space_conflict);
            }
        }

        Ok(conflicts)
    }

    fn detect_cycle(&self, node: &PlanNode, plan: &MovePlan) -> Result<bool, FileMoverError> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        self.detect_cycle_recursive(node.id, plan, &mut visited, &mut path)
    }

    fn detect_cycle_recursive(
        &self,
        node_id: PlanNodeId,
        plan: &MovePlan,
        visited: &mut HashSet<PlanNodeId>,
        path: &mut Vec<PlanNodeId>,
    ) -> Result<bool, FileMoverError> {
        if path.contains(&node_id) {
            return Ok(true); // 循環を検出
        }

        if visited.contains(&node_id) {
            return Ok(false); // 既に検証済み
        }

        visited.insert(node_id);
        path.push(node_id);

        if let Some(node) = plan.nodes.get(&node_id) {
            for &child_id in &node.children {
                if self.detect_cycle_recursive(child_id, plan, visited, path)? {
                    return Ok(true);
                }
            }
        }

        path.pop();
        Ok(false)
    }

    fn check_disk_space(&self, _path: &std::path::Path, required_size: u64) -> Result<Option<Conflict>, FileMoverError> {
        // 簡易実装: 実際にはstatvfsやGetDiskFreeSpaceExWを使用
        // ここでは1GB未満の場合に容量不足とする
        let available_space = 1024 * 1024 * 1024; // 1GB (仮の値)
        
        if required_size > available_space {
            Ok(Some(Conflict::NoSpace {
                required: required_size,
                available: available_space,
            }))
        } else {
            Ok(None)
        }
    }

    fn handle_skip_change(
        &mut self,
        plan: &mut MovePlan,
        node_id: PlanNodeId,
        skip: bool,
    ) -> Result<ValidationDelta, FileMoverError> {
        let mut affected_nodes = vec![node_id];
        let mut resolved_conflicts = Vec::new();

        if let Some(node) = plan.nodes.get_mut(&node_id) {
            let old_kind = node.kind;
            
            if skip {
                node.kind = OpKind::Skip;
                // スキップすることで解決される衝突を記録
                resolved_conflicts.extend(node.conflicts.clone());
                node.conflicts.clear();
            } else {
                // 元の操作種別を復元（実際の実装では履歴が必要）
                node.kind = match old_kind {
                    OpKind::Skip => OpKind::Move, // デフォルトに戻す
                    other => other,
                };
            }
        }

        let summary_diff = self.calculate_summary_diff_for_node(plan, node_id)?;

        Ok(ValidationDelta {
            affected_nodes,
            new_conflicts: Vec::new(),
            resolved_conflicts,
            summary_diff,
        })
    }

    fn handle_policy_change(
        &mut self,
        plan: &mut MovePlan,
        node_id: PlanNodeId,
        _policy: filemover_types::ConflictPolicy,
    ) -> Result<ValidationDelta, FileMoverError> {
        // 衝突ポリシー変更の処理
        // 実際の実装では、ポリシーに基づいて衝突解決を再実行
        let affected_nodes = vec![node_id];
        
        Ok(ValidationDelta {
            affected_nodes,
            new_conflicts: Vec::new(),
            resolved_conflicts: Vec::new(),
            summary_diff: PlanSummaryDiff {
                count_dirs_delta: 0,
                count_files_delta: 0,
                total_bytes_delta: None,
                cross_volume_delta: 0,
                conflicts_delta: 0,
                warnings_delta: 0,
            },
        })
    }

    fn handle_rename_change(
        &mut self,
        plan: &mut MovePlan,
        node_id: PlanNodeId,
        new_name: String,
    ) -> Result<ValidationDelta, FileMoverError> {
        let affected_nodes = vec![node_id];
        let mut new_conflicts = Vec::new();

        let (old_path, new_path) = if let Some(node) = plan.nodes.get_mut(&node_id) {
            // 新しいパスを構築
            let new_path = node.path_after.with_file_name(&new_name);
            let old_path = node.path_after.clone();
            
            node.name_after = new_name;
            node.path_after = new_path.clone();

            // 名前変更による新しい衝突をチェック
            if new_path.exists() {
                new_conflicts.push(Conflict::NameExists {
                    existing_path: new_path.clone(),
                });
            }

            (old_path, new_path)
        } else {
            return Err(FileMoverError::InvalidNodeId(node_id.to_string()));
        };

        // 子ノードのパスも更新が必要（別のスコープで実行）
        self.update_child_paths(plan, node_id, &old_path, &new_path)?;

        let summary_diff = PlanSummaryDiff {
            count_dirs_delta: 0,
            count_files_delta: 0,
            total_bytes_delta: None,
            cross_volume_delta: 0,
            conflicts_delta: new_conflicts.len() as i64,
            warnings_delta: 0,
        };

        Ok(ValidationDelta {
            affected_nodes,
            new_conflicts,
            resolved_conflicts: Vec::new(),
            summary_diff,
        })
    }

    fn handle_exclude_change(
        &mut self,
        plan: &mut MovePlan,
        node_id: PlanNodeId,
    ) -> Result<ValidationDelta, FileMoverError> {
        let mut affected_nodes = vec![node_id];
        let mut resolved_conflicts = Vec::new();

        if let Some(node) = plan.nodes.get_mut(&node_id) {
            node.kind = OpKind::None;
            resolved_conflicts.extend(node.conflicts.clone());
            node.conflicts.clear();
        }

        // 子ノードも除外
        let child_ids: Vec<PlanNodeId> = plan.nodes.get(&node_id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        
        for child_id in child_ids {
            affected_nodes.push(child_id);
            if let Some(child) = plan.nodes.get_mut(&child_id) {
                child.kind = OpKind::None;
                resolved_conflicts.extend(child.conflicts.clone());
                child.conflicts.clear();
            }
        }

        let summary_diff = self.calculate_summary_diff_for_exclusion(plan, &affected_nodes)?;

        Ok(ValidationDelta {
            affected_nodes,
            new_conflicts: Vec::new(),
            resolved_conflicts,
            summary_diff,
        })
    }

    fn update_child_paths(
        &mut self,
        plan: &mut MovePlan,
        parent_id: PlanNodeId,
        old_parent_path: &std::path::Path,
        new_parent_path: &std::path::Path,
    ) -> Result<(), FileMoverError> {
        let child_ids: Vec<PlanNodeId> = plan.nodes.get(&parent_id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
            
        for child_id in child_ids {
            if let Some(child) = plan.nodes.get_mut(&child_id) {
                // 子のパスを更新
                if child.path_after.starts_with(old_parent_path) {
                    let relative = child.path_after.strip_prefix(old_parent_path).unwrap();
                    child.path_after = new_parent_path.join(relative);
                }
            }
            
            // 再帰的に孫ノードも更新
            self.update_child_paths(plan, child_id, old_parent_path, new_parent_path)?;
        }
        
        Ok(())
    }

    fn recalculate_summary(&self, plan: &MovePlan) -> Result<PlanSummaryDiff, FileMoverError> {
        let mut new_summary = PlanSummary::default();
        
        for node in plan.nodes.values() {
            if !matches!(node.kind, OpKind::Skip | OpKind::None) {
                if node.is_dir {
                    new_summary.count_dirs += 1;
                } else {
                    new_summary.count_files += 1;
                }
                
                if let Some(size) = node.size_bytes {
                    new_summary.total_bytes = Some(
                        new_summary.total_bytes.unwrap_or(0) + size
                    );
                }
                
                new_summary.conflicts += node.conflicts.len() as u64;
                new_summary.warnings += node.warnings.len() as u64;
                
                // クロスボリューム検出（簡易実装）
                if self.is_cross_volume(&node.path_before, &node.path_after) {
                    new_summary.cross_volume += 1;
                }
            }
        }

        let old_summary = &plan.summary;
        
        Ok(PlanSummaryDiff {
            count_dirs_delta: new_summary.count_dirs as i64 - old_summary.count_dirs as i64,
            count_files_delta: new_summary.count_files as i64 - old_summary.count_files as i64,
            total_bytes_delta: match (new_summary.total_bytes, old_summary.total_bytes) {
                (Some(new), Some(old)) => Some(new as i64 - old as i64),
                (Some(new), None) => Some(new as i64),
                (None, Some(old)) => Some(-(old as i64)),
                (None, None) => None,
            },
            cross_volume_delta: new_summary.cross_volume as i64 - old_summary.cross_volume as i64,
            conflicts_delta: new_summary.conflicts as i64 - old_summary.conflicts as i64,
            warnings_delta: new_summary.warnings as i64 - old_summary.warnings as i64,
        })
    }

    fn calculate_summary_diff_for_node(
        &self,
        _plan: &MovePlan,
        _node_id: PlanNodeId,
    ) -> Result<PlanSummaryDiff, FileMoverError> {
        // 簡易実装
        Ok(PlanSummaryDiff {
            count_dirs_delta: 0,
            count_files_delta: 0,
            total_bytes_delta: None,
            cross_volume_delta: 0,
            conflicts_delta: 0,
            warnings_delta: 0,
        })
    }

    fn calculate_summary_diff_for_exclusion(
        &self,
        plan: &MovePlan,
        excluded_nodes: &[PlanNodeId],
    ) -> Result<PlanSummaryDiff, FileMoverError> {
        let mut dirs_delta = 0i64;
        let mut files_delta = 0i64;
        let mut bytes_delta = 0i64;
        let mut conflicts_delta = 0i64;
        let mut warnings_delta = 0i64;

        for &node_id in excluded_nodes {
            if let Some(node) = plan.nodes.get(&node_id) {
                if node.is_dir {
                    dirs_delta -= 1;
                } else {
                    files_delta -= 1;
                }
                
                if let Some(size) = node.size_bytes {
                    bytes_delta -= size as i64;
                }
                
                conflicts_delta -= node.conflicts.len() as i64;
                warnings_delta -= node.warnings.len() as i64;
            }
        }

        Ok(PlanSummaryDiff {
            count_dirs_delta: dirs_delta,
            count_files_delta: files_delta,
            total_bytes_delta: if bytes_delta != 0 { Some(bytes_delta) } else { None },
            cross_volume_delta: 0, // 除外による変更は計算しない
            conflicts_delta,
            warnings_delta,
        })
    }

    fn is_cross_volume(&self, source: &std::path::Path, dest: &std::path::Path) -> bool {
        // 簡易実装: ドライブレターが異なるかチェック
        let source_drive = source.components().next();
        let dest_drive = dest.components().next();
        
        source_drive != dest_drive
    }
}

impl Default for PlanValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cross_volume_detection() {
        let validator = PlanValidator::new();
        
        let source_c = PathBuf::from("C:\\Source");
        let dest_c = PathBuf::from("C:\\Dest");
        let dest_d = PathBuf::from("D:\\Dest");
        
        assert!(!validator.is_cross_volume(&source_c, &dest_c));
        assert!(validator.is_cross_volume(&source_c, &dest_d));
    }

    #[test]
    fn test_cycle_detection() {
        let mut plan = MovePlan {
            roots: Vec::new(),
            nodes: HashMap::new(),
            summary: PlanSummary::default(),
        };

        // 循環参照を作成
        let node1_id = PlanNodeId::new();
        let node2_id = PlanNodeId::new();
        
        let node1 = PlanNode {
            id: node1_id,
            is_dir: true,
            name_before: "node1".to_string(),
            path_before: PathBuf::from("C:\\node1"),
            name_after: "node1".to_string(),
            path_after: PathBuf::from("D:\\node1"),
            kind: OpKind::Move,
            size_bytes: None,
            warnings: Vec::new(),
            conflicts: Vec::new(),
            children: vec![node2_id],
            rule_id: None,
        };

        let node2 = PlanNode {
            id: node2_id,
            is_dir: true,
            name_before: "node2".to_string(),
            path_before: PathBuf::from("C:\\node2"),
            name_after: "node2".to_string(),
            path_after: PathBuf::from("D:\\node2"),
            kind: OpKind::Move,
            size_bytes: None,
            warnings: Vec::new(),
            conflicts: Vec::new(),
            children: vec![node1_id], // 循環参照
            rule_id: None,
        };

        plan.nodes.insert(node1_id, node1);
        plan.nodes.insert(node2_id, node2);

        let validator = PlanValidator::new();
        let has_cycle = validator.detect_cycle(plan.nodes.get(&node1_id).unwrap(), &plan).unwrap();
        assert!(has_cycle);
    }
}