import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { ScanSession } from '../types';
import { sessionManager } from '../services/sessionManager';

function ScanResultsPage() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<ScanSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedHits, setSelectedHits] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!sessionId) return;

    const pollSession = async () => {
      try {
        const sessionData = await invoke<ScanSession>('get_scan_session', { sessionId });
        setSession(sessionData);
        
        if (sessionData.status === 'Running') {
          setTimeout(pollSession, 1000);
        } else {
          setIsLoading(false);
        }
      } catch (error) {
        console.error('Tauri API not available, trying browser implementation:', error);
        
        // Try browser-based session manager
        const browserSession = sessionManager.getScanSession(sessionId);
        if (browserSession) {
          setSession(browserSession);
          
          if (browserSession.status === 'Running') {
            setTimeout(pollSession, 1000);
          } else {
            setIsLoading(false);
          }
          return;
        }
        
        // Demo mode fallback - create mock session data
        if (sessionId?.startsWith('demo-')) {
          const mockSession: ScanSession = {
            id: sessionId,
            roots: ['C:\\Users\\Demo\\Documents', 'C:\\Users\\Demo\\Downloads'],
            status: 'Completed',
            progress: {
              current_item: undefined,
              completed_ops: 150,
              total_ops: 150,
              bytes_processed: 1024 * 1024 * 256, // 256MB
              total_bytes: 1024 * 1024 * 256,
              current_speed: undefined,
              eta: undefined,
            },
            results: [
              {
                path: 'C:\\Users\\Demo\\Documents\\Photos\\Vacation2023',
                name: 'Vacation2023',
                matched_rule: 'Photo Rule',
                dest_preview: 'C:\\Users\\Demo\\Pictures\\Organized\\2023\\Vacation2023',
                warnings: [],
                size_bytes: 1024 * 1024 * 50, // 50MB
              },
              {
                path: 'C:\\Users\\Demo\\Downloads\\Software\\old_installers',
                name: 'old_installers',
                matched_rule: 'Software Rule',
                dest_preview: 'C:\\Users\\Demo\\Archive\\Software\\old_installers',
                warnings: ['LongPath'],
                size_bytes: 1024 * 1024 * 120, // 120MB
              },
              {
                path: 'C:\\Users\\Demo\\Documents\\Projects\\WebProject',
                name: 'WebProject',
                matched_rule: undefined,
                dest_preview: undefined,
                warnings: ['AccessDenied'],
                size_bytes: 1024 * 1024 * 86, // 86MB
              },
            ],
            error: undefined,
          };
          setSession(mockSession);
        }
        setIsLoading(false);
      }
    };

    pollSession();
  }, [sessionId]);

  const toggleSelection = (path: string) => {
    const newSelected = new Set(selectedHits);
    if (newSelected.has(path)) {
      newSelected.delete(path);
    } else {
      newSelected.add(path);
    }
    setSelectedHits(newSelected);
  };

  const selectAll = () => {
    if (!session?.results) return;
    const allPaths = new Set(session.results.map(hit => hit.path));
    setSelectedHits(allPaths);
  };

  const selectNone = () => {
    setSelectedHits(new Set());
  };

  const createPlan = async () => {
    if (!sessionId || selectedHits.size === 0) return;

    try {
      setIsLoading(true);
      const planSessionId = await invoke<string>('create_plan_session', {
        scanId: sessionId,
        selectedPaths: Array.from(selectedHits),
      });
      
      await invoke('start_planning', { sessionId: planSessionId });
      navigate(`/plan/${planSessionId}`);
    } catch (error) {
      console.error('Tauri API not available, trying browser implementation:', error);
      
      try {
        // Use real browser-based implementation
        const planSessionId = sessionManager.createPlanSession(sessionId, Array.from(selectedHits));
        
        // Start planning in background
        sessionManager.startPlanning(planSessionId)
          .catch(err => console.error('Planning failed:', err));
        
        navigate(`/plan/${planSessionId}`);
      } catch (browserError) {
        console.error('Browser implementation failed:', browserError);
        // Demo mode fallback - create demo plan session
        const demoPlanSessionId = `plan-demo-${Date.now()}`;
        alert(`フォールバック: デモモードで動作します (プランID: ${demoPlanSessionId})`);
        navigate(`/plan/${demoPlanSessionId}`);
      }
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

  const getWarningBadge = (warnings: string[]) => {
    if (warnings.length === 0) return null;
    
    const warningTypes = {
      'LongPath': { label: '長いパス', color: 'badge-warning' },
      'AclDiffers': { label: 'ACL差異', color: 'badge-info' },
      'Offline': { label: 'オフライン', color: 'badge-error' },
      'AccessDenied': { label: 'アクセス拒否', color: 'badge-error' },
      'Junction': { label: 'ジャンクション', color: 'badge-info' },
      'CrossVolume': { label: 'ボリューム間', color: 'badge-warning' },
    };

    return (
      <div className="flex flex-wrap gap-1">
        {warnings.map((warning, index) => {
          const config = warningTypes[warning as keyof typeof warningTypes] || 
            { label: warning, color: 'badge-info' };
          return (
            <span key={index} className={`${config.color} text-xs`}>
              {config.label}
            </span>
          );
        })}
      </div>
    );
  };

  if (isLoading && !session) {
    return (
      <div className="flex items-center justify-center min-h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">スキャン結果を読み込んでいます...</p>
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
          <h2 className="text-2xl font-bold text-gray-900">スキャン実行中</h2>
          <p className="text-gray-600 mt-2">フォルダをスキャンしています...</p>
        </div>

        {session.progress && (
          <div className="card">
            <div className="mb-4">
              <div className="flex justify-between text-sm text-gray-600 mb-2">
                <span>進行状況</span>
                <span>{session.progress.completed_ops} / {session.progress.total_ops}</span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className="bg-primary-600 h-2 rounded-full transition-all duration-300"
                  style={{
                    width: `${(session.progress.completed_ops / session.progress.total_ops) * 100}%`
                  }}
                ></div>
              </div>
            </div>
            
            {session.progress.current_item && (
              <p className="text-sm text-gray-700">
                現在: <span className="font-mono">{session.progress.current_item}</span>
              </p>
            )}
            
            {session.progress.current_speed && (
              <p className="text-sm text-gray-500 mt-2">
                速度: {session.progress.current_speed.toFixed(1)} items/sec
              </p>
            )}
          </div>
        )}
      </div>
    );
  }

  if (session.status === 'Failed') {
    return (
      <div className="text-center">
        <p className="text-red-600 mb-4">スキャンが失敗しました: {session.error}</p>
        <button onClick={() => navigate('/')} className="btn-primary">
          最初に戻る
        </button>
      </div>
    );
  }

  const results = session.results || [];
  const totalSize = results.reduce((sum, hit) => sum + (hit.size_bytes || 0), 0);
  const selectedSize = results
    .filter(hit => selectedHits.has(hit.path))
    .reduce((sum, hit) => sum + (hit.size_bytes || 0), 0);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900">スキャン結果</h2>
        <p className="text-gray-600 mt-2">
          {results.length}個のフォルダが見つかりました (合計: {formatBytes(totalSize)})
        </p>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">検出フォルダ</h3>
          <p className="text-3xl font-bold text-primary-600">{results.length}</p>
          <p className="text-sm text-gray-500 mt-1">個のフォルダ</p>
        </div>
        
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">選択中</h3>
          <p className="text-3xl font-bold text-green-600">{selectedHits.size}</p>
          <p className="text-sm text-gray-500 mt-1">{formatBytes(selectedSize)}</p>
        </div>
        
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">警告あり</h3>
          <p className="text-3xl font-bold text-yellow-600">
            {results.filter(hit => hit.warnings.length > 0).length}
          </p>
          <p className="text-sm text-gray-500 mt-1">個のフォルダ</p>
        </div>
      </div>

      {/* Selection Controls */}
      <div className="card">
        <div className="flex justify-between items-center">
          <div className="space-x-4">
            <button onClick={selectAll} className="btn-secondary btn-sm">
              すべて選択
            </button>
            <button onClick={selectNone} className="btn-secondary btn-sm">
              選択解除
            </button>
          </div>
          
          <button
            onClick={createPlan}
            disabled={selectedHits.size === 0 || isLoading}
            className="btn-primary"
          >
            {isLoading ? 'プラン作成中...' : `プランを作成 (${selectedHits.size}件)`}
          </button>
        </div>
      </div>

      {/* Results Table */}
      <div className="card">
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="w-12 px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  選択
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  パス
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  移動先プレビュー
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  サイズ
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  警告
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {results.map((hit) => (
                <tr
                  key={hit.path}
                  className={`hover:bg-gray-50 ${
                    selectedHits.has(hit.path) ? 'bg-blue-50' : ''
                  }`}
                >
                  <td className="px-6 py-4 whitespace-nowrap">
                    <input
                      type="checkbox"
                      checked={selectedHits.has(hit.path)}
                      onChange={() => toggleSelection(hit.path)}
                      className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                    />
                  </td>
                  <td className="px-6 py-4">
                    <div className="text-sm font-mono text-gray-900 break-all">
                      {hit.path}
                    </div>
                    <div className="text-xs text-gray-500 mt-1">
                      フォルダ名: {hit.name}
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <div className="text-sm text-gray-900 break-all">
                      {hit.dest_preview || 'N/A'}
                    </div>
                    {hit.matched_rule && (
                      <div className="text-xs text-blue-600 mt-1">
                        ルール: {hit.matched_rule}
                      </div>
                    )}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                    {formatBytes(hit.size_bytes)}
                  </td>
                  <td className="px-6 py-4">
                    {getWarningBadge(hit.warnings)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          
          {results.length === 0 && (
            <div className="text-center py-12">
              <p className="text-gray-500">スキャン結果がありません。</p>
              <button onClick={() => navigate('/')} className="btn-primary mt-4">
                新しいスキャンを開始
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default ScanResultsPage;