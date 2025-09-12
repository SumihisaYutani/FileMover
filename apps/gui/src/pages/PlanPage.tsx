import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { PlanSession, MovePlan, PlanNode, Conflict } from '../types';

function PlanPage() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<PlanSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!sessionId) return;

    const pollSession = async () => {
      try {
        const sessionData = await invoke<PlanSession>('get_plan_session', { sessionId });
        setSession(sessionData);
        
        if (sessionData.status === 'Running') {
          setTimeout(pollSession, 1000);
        } else {
          setIsLoading(false);
        }
      } catch (error) {
        console.error('Tauri API not available, using demo mode:', error);
        
        // Demo mode - create mock plan session
        if (sessionId?.startsWith('plan-demo-')) {
          const mockPlanSession: PlanSession = {
            id: sessionId,
            scan_id: 'demo-scan-123',
            status: 'Completed',
            plan: {
              roots: ['root1', 'root2'],
              nodes: {
                'root1': {
                  id: 'root1',
                  is_dir: true,
                  name_before: 'Vacation2023',
                  path_before: 'C:\\Users\\Demo\\Documents\\Photos\\Vacation2023',
                  name_after: 'Vacation2023',
                  path_after: 'C:\\Users\\Demo\\Pictures\\Organized\\2023\\Vacation2023',
                  kind: 'Move',
                  size_bytes: 1024 * 1024 * 50,
                  warnings: [],
                  conflicts: [],
                  children: [],
                  rule_id: 'photo-rule',
                },
                'root2': {
                  id: 'root2',
                  is_dir: true,
                  name_before: 'old_installers',
                  path_before: 'C:\\Users\\Demo\\Downloads\\Software\\old_installers',
                  name_after: 'old_installers',
                  path_after: 'C:\\Users\\Demo\\Archive\\Software\\old_installers',
                  kind: 'Move',
                  size_bytes: 1024 * 1024 * 120,
                  warnings: ['LongPath'],
                  conflicts: [
                    {
                      type: 'NameExists',
                      existing_path: 'C:\\Users\\Demo\\Archive\\Software\\old_installers_backup',
                    }
                  ],
                  children: [],
                  rule_id: 'software-rule',
                },
              },
              summary: {
                count_dirs: 2,
                count_files: 156,
                total_bytes: 1024 * 1024 * 170,
                cross_volume: 0,
                conflicts: 1,
                warnings: 1,
              },
            },
            error: undefined,
          };
          setSession(mockPlanSession);
        }
        setIsLoading(false);
      }
    };

    pollSession();
  }, [sessionId]);

  const toggleExpanded = (nodeId: string) => {
    const newExpanded = new Set(expandedNodes);
    if (newExpanded.has(nodeId)) {
      newExpanded.delete(nodeId);
    } else {
      newExpanded.add(nodeId);
    }
    setExpandedNodes(newExpanded);
  };

  const executePlan = async () => {
    if (!sessionId) return;

    try {
      setIsLoading(true);
      const executionSessionId = await invoke<string>('create_execution_session', {
        planId: sessionId,
      });
      
      await invoke('start_execution', { sessionId: executionSessionId });
      navigate(`/execution/${executionSessionId}`);
    } catch (error) {
      console.error('Tauri API not available, using demo mode:', error);
      // Demo mode - create demo execution session
      const demoExecutionSessionId = `execution-demo-${Date.now()}`;
      alert(`デモモード: 実行を開始します (実行ID: ${demoExecutionSessionId})`);
      navigate(`/execution/${demoExecutionSessionId}`);
    } finally {
      setIsLoading(false);
    }
  };

  const formatBytes = (bytes?: number) => {
    if (!bytes) return 'N/A';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = bytes;
    let unitIndex = 0;
    
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    
    return `${size.toFixed(1)} ${units[unitIndex]}`;
  };

  const getOpKindLabel = (kind: string) => {
    const labels = {
      'Move': '移動',
      'CopyDelete': 'コピー&削除',
      'Rename': 'リネーム',
      'Skip': 'スキップ',
      'None': '変更なし',
    };
    return labels[kind as keyof typeof labels] || kind;
  };

  const getOpKindColor = (kind: string) => {
    const colors = {
      'Move': 'text-blue-600',
      'CopyDelete': 'text-orange-600',
      'Rename': 'text-green-600',
      'Skip': 'text-gray-600',
      'None': 'text-gray-400',
    };
    return colors[kind as keyof typeof colors] || 'text-gray-600';
  };

  const getConflictBadge = (conflicts: Conflict[]) => {
    if (conflicts.length === 0) return null;
    
    const conflictTypes = {
      'NameExists': { label: '名前競合', color: 'badge-warning' },
      'CycleDetected': { label: 'サイクル検出', color: 'badge-error' },
      'DestInsideSource': { label: '移動先が移動元内', color: 'badge-error' },
      'NoSpace': { label: '容量不足', color: 'badge-error' },
      'Permission': { label: '権限不足', color: 'badge-error' },
    };

    return (
      <div className="flex flex-wrap gap-1">
        {conflicts.map((conflict, index) => {
          const config = conflictTypes[conflict.type as keyof typeof conflictTypes] || 
            { label: conflict.type, color: 'badge-error' };
          return (
            <span key={index} className={`${config.color} text-xs`}>
              {config.label}
            </span>
          );
        })}
      </div>
    );
  };

  const renderNode = (node: PlanNode, plan: MovePlan, depth: number = 0) => {
    const hasChildren = node.children.length > 0;
    const isExpanded = expandedNodes.has(node.id);
    const hasConflicts = node.conflicts.length > 0;
    const hasWarnings = node.warnings.length > 0;

    return (
      <div key={node.id} className={`${depth > 0 ? 'ml-8' : ''}`}>
        <div className={`p-3 border rounded-md ${
          hasConflicts ? 'border-red-200 bg-red-50' : 
          hasWarnings ? 'border-yellow-200 bg-yellow-50' : 
          'border-gray-200 bg-white'
        }`}>
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3 flex-1">
              {hasChildren && (
                <button
                  onClick={() => toggleExpanded(node.id)}
                  className="text-gray-400 hover:text-gray-600"
                >
                  {isExpanded ? '▼' : '▶'}
                </button>
              )}
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2">
                  <span className={`text-sm font-medium ${getOpKindColor(node.kind)}`}>
                    {getOpKindLabel(node.kind)}
                  </span>
                  <span className="text-xs text-gray-500">
                    {node.is_dir ? 'フォルダ' : 'ファイル'}
                  </span>
                  {node.size_bytes && (
                    <span className="text-xs text-gray-500">
                      ({formatBytes(node.size_bytes)})
                    </span>
                  )}
                </div>
                
                <div className="mt-1 space-y-1">
                  <div className="text-sm text-gray-900 font-mono break-all">
                    変更前: {node.path_before}
                  </div>
                  {node.path_after !== node.path_before && (
                    <div className="text-sm text-green-700 font-mono break-all">
                      変更後: {node.path_after}
                    </div>
                  )}
                </div>
                
                {node.rule_id && (
                  <div className="text-xs text-blue-600 mt-1">
                    適用ルール: {node.rule_id}
                  </div>
                )}
              </div>
            </div>
            
            <div className="ml-4 space-y-1">
              {getConflictBadge(node.conflicts)}
              {node.warnings.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {node.warnings.map((warning, index) => (
                    <span key={index} className="badge-warning text-xs">
                      {warning}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
          
          {node.conflicts.length > 0 && (
            <div className="mt-2 pt-2 border-t border-red-200">
              <h4 className="text-sm font-medium text-red-800 mb-1">競合の詳細:</h4>
              {node.conflicts.map((conflict, index) => (
                <div key={index} className="text-sm text-red-700">
                  {conflict.type === 'NameExists' && conflict.existing_path && (
                    <p>既存のファイル/フォルダと名前が競合: {conflict.existing_path}</p>
                  )}
                  {conflict.type === 'NoSpace' && (
                    <p>
                      容量不足: 必要 {formatBytes(conflict.required)} / 
                      利用可能 {formatBytes(conflict.available)}
                    </p>
                  )}
                  {conflict.type === 'Permission' && (
                    <p>権限不足: {conflict.required_permission} が必要です</p>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
        
        {hasChildren && isExpanded && (
          <div className="mt-2">
            {node.children.map(childId => {
              const childNode = plan.nodes[childId];
              return childNode ? renderNode(childNode, plan, depth + 1) : null;
            })}
          </div>
        )}
      </div>
    );
  };

  if (isLoading && !session) {
    return (
      <div className="flex items-center justify-center min-h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">移動プランを作成しています...</p>
        </div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="text-center">
        <p className="text-red-600">セッションが見つかりません。</p>
        <button onClick={() => navigate('/')} className="btn-primary mt-4">
          最初に戻る
        </button>
      </div>
    );
  }

  if (session.status === 'Running') {
    return (
      <div className="space-y-6">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">プラン作成中</h2>
          <p className="text-gray-600 mt-2">移動プランを計算しています...</p>
        </div>
        
        <div className="card">
          <div className="animate-pulse">
            <div className="h-4 bg-gray-200 rounded w-3/4 mb-4"></div>
            <div className="h-4 bg-gray-200 rounded w-1/2"></div>
          </div>
        </div>
      </div>
    );
  }

  if (session.status === 'Failed') {
    return (
      <div className="text-center">
        <p className="text-red-600 mb-4">プラン作成が失敗しました: {session.error}</p>
        <button onClick={() => navigate('/')} className="btn-primary">
          最初に戻る
        </button>
      </div>
    );
  }

  const plan = session.plan;
  if (!plan) {
    return (
      <div className="text-center">
        <p className="text-gray-600">プランが利用できません。</p>
        <button onClick={() => navigate('/')} className="btn-primary mt-4">
          最初に戻る
        </button>
      </div>
    );
  }

  const hasConflicts = plan.summary.conflicts > 0;
  const hasWarnings = plan.summary.warnings > 0;
  const rootNodes = plan.roots.map(rootId => plan.nodes[rootId]).filter(Boolean);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900">移動プラン確認</h2>
        <p className="text-gray-600 mt-2">
          以下の操作が実行されます。内容を確認してから実行してください。
        </p>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">フォルダ</h3>
          <p className="text-3xl font-bold text-blue-600">{plan.summary.count_dirs}</p>
          <p className="text-sm text-gray-500 mt-1">個</p>
        </div>
        
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">ファイル</h3>
          <p className="text-3xl font-bold text-green-600">{plan.summary.count_files}</p>
          <p className="text-sm text-gray-500 mt-1">個</p>
        </div>
        
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">競合</h3>
          <p className={`text-3xl font-bold ${hasConflicts ? 'text-red-600' : 'text-gray-400'}`}>
            {plan.summary.conflicts}
          </p>
          <p className="text-sm text-gray-500 mt-1">件</p>
        </div>
        
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">警告</h3>
          <p className={`text-3xl font-bold ${hasWarnings ? 'text-yellow-600' : 'text-gray-400'}`}>
            {plan.summary.warnings}
          </p>
          <p className="text-sm text-gray-500 mt-1">件</p>
        </div>
      </div>

      {/* Additional Summary Info */}
      <div className="card">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div>
            <span className="font-medium text-gray-700">総サイズ:</span>
            <span className="ml-2 text-gray-900">{formatBytes(plan.summary.total_bytes)}</span>
          </div>
          <div>
            <span className="font-medium text-gray-700">ボリューム間移動:</span>
            <span className="ml-2 text-gray-900">{plan.summary.cross_volume}件</span>
          </div>
          <div>
            <span className="font-medium text-gray-700">スキャン対象ルート:</span>
            <span className="ml-2 text-gray-900">{plan.roots.length}個</span>
          </div>
        </div>
      </div>

      {/* Warnings and Conflicts Alert */}
      {(hasConflicts || hasWarnings) && (
        <div className={`card ${hasConflicts ? 'border-red-200 bg-red-50' : 'border-yellow-200 bg-yellow-50'}`}>
          <div className="flex items-start">
            <div className="flex-shrink-0">
              {hasConflicts ? (
                <div className="w-5 h-5 text-red-400">⚠</div>
              ) : (
                <div className="w-5 h-5 text-yellow-400">⚠</div>
              )}
            </div>
            <div className="ml-3">
              <h3 className={`text-sm font-medium ${hasConflicts ? 'text-red-800' : 'text-yellow-800'}`}>
                {hasConflicts ? '重要な警告: 競合が検出されました' : '警告があります'}
              </h3>
              <p className={`mt-1 text-sm ${hasConflicts ? 'text-red-700' : 'text-yellow-700'}`}>
                {hasConflicts 
                  ? '一部の操作で競合が発生しています。実行前に競合を解決することを強く推奨します。' 
                  : '一部の操作で警告があります。内容を確認してから実行してください。'
                }
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Plan Tree */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">操作詳細</h3>
        
        <div className="space-y-3">
          {rootNodes.map(node => renderNode(node, plan))}
        </div>
        
        {rootNodes.length === 0 && (
          <p className="text-gray-500 text-center py-8">実行可能な操作がありません。</p>
        )}
      </div>

      {/* Action Buttons */}
      <div className="flex justify-between">
        <button
          onClick={() => navigate(-1)}
          className="btn-secondary"
        >
          戻る
        </button>
        
        <div className="space-x-4">
          {hasConflicts && (
            <span className="text-sm text-red-600">
              ⚠ 競合があるため、実行には注意が必要です
            </span>
          )}
          <button
            onClick={executePlan}
            disabled={isLoading}
            className={`${hasConflicts ? 'btn-danger' : 'btn-primary'} btn-lg`}
          >
            {isLoading ? '実行開始中...' : '実行開始'}
          </button>
        </div>
      </div>
    </div>
  );
}

export default PlanPage;