// Re-export types that are shared between frontend and backend
export interface Config {
  roots: string[];
  rules: Rule[];
  options: ScanOptions;
  profiles: string[];
}

export interface Rule {
  id: string;
  enabled: boolean;
  pattern: PatternSpec;
  dest_root: string;
  template: string;
  policy: ConflictPolicy;
  label?: string;
  priority: number;
}

export interface PatternSpec {
  kind: PatternKind;
  value: string;
  is_exclude: boolean;
  case_insensitive: boolean;
}

export type PatternKind = 'Glob' | 'Regex' | 'Contains';
export type ConflictPolicy = 'AutoRename' | 'Skip' | 'Overwrite';

export interface ScanOptions {
  normalization: NormalizationOptions;
  follow_junctions: boolean;
  system_protections: boolean;
  max_depth?: number;
  excluded_paths: string[];
  parallel_threads?: number;
}

export interface NormalizationOptions {
  normalize_unicode: boolean;
  normalize_width: boolean;
  strip_diacritics: boolean;
  normalize_case: boolean;
}

export interface FolderHit {
  path: string;
  name: string;
  matched_rule?: string;
  dest_preview?: string;
  warnings: Warning[];
  size_bytes?: number;
}

export type Warning = 'LongPath' | 'AclDiffers' | 'Offline' | 'AccessDenied' | 'Junction' | 'CrossVolume';

export interface MovePlan {
  roots: string[];
  nodes: Record<string, PlanNode>;
  summary: PlanSummary;
}

export interface PlanNode {
  id: string;
  is_dir: boolean;
  name_before: string;
  path_before: string;
  name_after: string;
  path_after: string;
  kind: OpKind;
  size_bytes?: number;
  warnings: Warning[];
  conflicts: Conflict[];
  children: string[];
  rule_id?: string;
}

export type OpKind = 'Move' | 'CopyDelete' | 'Rename' | 'Skip' | 'None';

export interface Conflict {
  type: 'NameExists' | 'CycleDetected' | 'DestInsideSource' | 'NoSpace' | 'Permission';
  existing_path?: string;
  required?: number;
  available?: number;
  required_permission?: Permission;
}

export type Permission = 'Administrator' | 'FileSystemWrite' | 'NetworkAccess';

export interface PlanSummary {
  count_dirs: number;
  count_files: number;
  total_bytes?: number;
  cross_volume: number;
  conflicts: number;
  warnings: number;
}

// Frontend-specific types
export interface ScanSession {
  id: string;
  roots: string[];
  status: SessionStatus;
  progress?: Progress;
  results?: FolderHit[];
  error?: string;
}

export interface PlanSession {
  id: string;
  scan_id?: string;
  status: SessionStatus;
  plan?: MovePlan;
  error?: string;
}

export interface ExecutionSession {
  id: string;
  plan_id: string;
  status: SessionStatus;
  progress?: Progress;
  journal_path?: string;
  error?: string;
}

export type SessionStatus = 'Created' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';

export interface Progress {
  current_item?: string;
  completed_ops: number;
  total_ops: number;
  bytes_processed: number;
  total_bytes?: number;
  current_speed?: number;
  eta?: number;
}

export interface SimulationReport {
  success_estimate: number;
  conflicts_remaining: number;
  skipped_count: number;
  estimated_duration: number;
}

export interface PathValidation {
  is_valid: boolean;
  exists: boolean;
  is_directory: boolean;
  is_readable: boolean;
  is_writable: boolean;
  is_long_path: boolean;
  is_network_path: boolean;
  is_system_protected: boolean;
  warnings: string[];
  errors: string[];
}

export interface SystemInfo {
  os_type: string;
  arch: string;
  long_path_support: boolean;
  available_drives: DriveInfo[];
}

export interface DriveInfo {
  path: string;
  label: string;
  drive_type: DriveType;
  total_space?: number;
  free_space?: number;
}

export type DriveType = 'Fixed' | 'Removable' | 'Network' | 'CD' | 'Ram' | 'Unknown';

export interface UndoResult {
  total_operations: number;
  undone_operations: number;
  failed_operations: number;
  skipped_operations: number;
  errors: string[];
}

export interface JournalValidation {
  is_valid: boolean;
  total_entries: number;
  successful_entries: number;
  failed_entries: number;
  skipped_entries: number;
  undoable_entries: number;
  issues: string[];
}