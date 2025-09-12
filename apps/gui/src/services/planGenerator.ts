import { PlanSession, MovePlan, PlanNode, FolderHit, Conflict, PlanSummary } from '../types';

export class PlanGenerator {
  private session: PlanSession;
  private folderHits: FolderHit[];
  private nodes: Record<string, PlanNode> = {};

  constructor(sessionId: string, folderHits: FolderHit[]) {
    this.session = {
      id: sessionId,
      scan_id: undefined,
      status: 'Created',
      plan: undefined,
      error: undefined,
    };
    this.folderHits = folderHits;
  }

  async generatePlan(): Promise<void> {
    this.session.status = 'Running';

    try {
      await this.simulateProcessingDelay();
      
      const rootIds: string[] = [];
      let totalBytes = 0;
      let countDirs = 0;
      let countFiles = 0;
      let crossVolume = 0;
      let totalConflicts = 0;
      let totalWarnings = 0;

      for (let i = 0; i < this.folderHits.length; i++) {
        const hit = this.folderHits[i];
        const nodeId = `node_${i}`;
        rootIds.push(nodeId);

        // Analyze conflicts
        const conflicts = this.analyzeConflicts(hit);
        totalConflicts += conflicts.length;
        totalWarnings += hit.warnings.length;

        // Estimate file count (simulate)
        const estimatedFiles = Math.floor(Math.random() * 50) + 10;
        countFiles += estimatedFiles;
        countDirs += 1;

        if (hit.size_bytes) {
          totalBytes += hit.size_bytes;
        }

        // Check for cross-volume moves
        if (hit.dest_preview && this.isCrossVolume(hit.path, hit.dest_preview)) {
          crossVolume++;
        }

        const node: PlanNode = {
          id: nodeId,
          is_dir: true,
          name_before: hit.name,
          path_before: hit.path,
          name_after: hit.name,
          path_after: hit.dest_preview || hit.path,
          kind: this.determineOperationKind(hit),
          size_bytes: hit.size_bytes,
          warnings: hit.warnings,
          conflicts,
          children: [],
          rule_id: hit.matched_rule,
        };

        this.nodes[nodeId] = node;
      }

      const summary: PlanSummary = {
        count_dirs: countDirs,
        count_files: countFiles,
        total_bytes: totalBytes,
        cross_volume: crossVolume,
        conflicts: totalConflicts,
        warnings: totalWarnings,
      };

      const plan: MovePlan = {
        roots: rootIds,
        nodes: this.nodes,
        summary,
      };

      this.session.status = 'Completed';
      this.session.plan = plan;

    } catch (error) {
      this.session.status = 'Failed';
      this.session.error = error instanceof Error ? error.message : String(error);
    }
  }

  private async simulateProcessingDelay(): Promise<void> {
    // Simulate plan generation time
    await new Promise(resolve => setTimeout(resolve, 800 + Math.random() * 1200));
  }

  private analyzeConflicts(hit: FolderHit): Conflict[] {
    const conflicts: Conflict[] = [];

    // Simulate name conflicts
    if (hit.dest_preview && Math.random() < 0.3) {
      conflicts.push({
        type: 'NameExists',
        existing_path: `${hit.dest_preview}_backup`,
      });
    }

    // Simulate permission conflicts
    if (hit.warnings.includes('AccessDenied')) {
      conflicts.push({
        type: 'Permission',
        required_permission: 'Administrator',
      });
    }

    // Simulate space conflicts for large files
    if (hit.size_bytes && hit.size_bytes > 1024 * 1024 * 100 && Math.random() < 0.2) {
      const required = hit.size_bytes;
      const available = Math.floor(required * (0.7 + Math.random() * 0.2)); // 70-90% available
      
      if (available < required) {
        conflicts.push({
          type: 'NoSpace',
          required,
          available,
        });
      }
    }

    // Check for potential cycles (simplified check)
    if (hit.dest_preview && hit.path.toLowerCase().includes(hit.dest_preview.toLowerCase())) {
      conflicts.push({
        type: 'DestInsideSource',
      });
    }

    return conflicts;
  }

  private determineOperationKind(hit: FolderHit): 'Move' | 'CopyDelete' | 'Rename' | 'Skip' | 'None' {
    if (!hit.dest_preview) {
      return 'Skip';
    }

    const conflicts = this.analyzeConflicts(hit);
    if (conflicts.length > 0) {
      return 'Skip';
    }

    if (hit.warnings.includes('AccessDenied')) {
      return 'Skip';
    }

    // Check if it's a cross-volume operation
    if (this.isCrossVolume(hit.path, hit.dest_preview)) {
      return 'CopyDelete';
    }

    // Check if it's just a rename (same directory)
    const sourcePath = hit.path.substring(0, hit.path.lastIndexOf('\\'));
    const destPath = hit.dest_preview.substring(0, hit.dest_preview.lastIndexOf('\\'));
    
    if (sourcePath.toLowerCase() === destPath.toLowerCase()) {
      return 'Rename';
    }

    return 'Move';
  }

  private isCrossVolume(sourcePath: string, destPath: string): boolean {
    // Extract drive letters (Windows-specific)
    const sourceMatch = sourcePath.match(/^([A-Za-z]):/);
    const destMatch = destPath.match(/^([A-Za-z]):/);
    
    if (sourceMatch && destMatch) {
      return sourceMatch[1].toLowerCase() !== destMatch[1].toLowerCase();
    }

    // For network paths or UNC paths, do a simple comparison
    const sourceRoot = sourcePath.split('\\')[0];
    const destRoot = destPath.split('\\')[0];
    
    return sourceRoot !== destRoot;
  }

  getSession(): PlanSession {
    return this.session;
  }
}