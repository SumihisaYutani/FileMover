import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { ExecutionSession, Progress } from '../types';

function ExecutionPage() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<ExecutionSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [canCancel, setCanCancel] = useState(true);

  useEffect(() => {
    if (!sessionId) return;

    const pollSession = async () => {
      try {
        const sessionData = await invoke<ExecutionSession>('get_execution_session', { sessionId });
        setSession(sessionData);
        
        if (sessionData.status === 'Running') {
          setTimeout(pollSession, 500); // Poll more frequently during execution
        } else {
          setIsLoading(false);
          setCanCancel(false);
          
          // Automatically navigate to results after completion
          if (sessionData.status === 'Completed') {
            setTimeout(() => {
              navigate(`/results/${sessionId}`);
            }, 2000);
          }
        }
      } catch (error) {
        console.error('Tauri API not available, using demo mode:', error);
        
        // Demo mode - create mock execution session
        if (sessionId?.startsWith('execution-demo-')) {
          const mockExecutionSession: ExecutionSession = {
            id: sessionId,
            plan_id: 'plan-demo-123',
            status: 'Running',
            progress: {
              current_item: 'C:\\Users\\Demo\\Documents\\Photos\\Vacation2023\\IMG_001.jpg',
              completed_ops: 45,
              total_ops: 156,
              bytes_processed: 1024 * 1024 * 78, // 78MB
              total_bytes: 1024 * 1024 * 170, // 170MB
              current_speed: 12.5,
              eta: 8,
            },
            journal_path: 'C:\\Users\\Demo\\AppData\\Local\\FileMover\\execution_journal.json',
            error: undefined,
          };
          setSession(mockExecutionSession);
          
          // Simulate progress updates
          let progress = 45;
          const progressInterval = setInterval(() => {
            progress += Math.floor(Math.random() * 5) + 1;
            if (progress >= 156) {
              progress = 156;
              clearInterval(progressInterval);
              // Complete the execution after a delay
              setTimeout(() => {
                setSession(prev => prev ? {
                  ...prev,
                  status: 'Completed',
                  progress: {
                    ...prev.progress!,
                    current_item: undefined,
                    completed_ops: 156,
                    current_speed: undefined,
                    eta: undefined,
                  }
                } : null);
                setCanCancel(false);
                
                // Auto-navigate to results
                setTimeout(() => {
                  navigate(`/results/${sessionId}`);
                }, 2000);
              }, 1000);
              return;
            }
            
            setSession(prev => prev ? {
              ...prev,
              progress: {
                ...prev.progress!,
                completed_ops: progress,
                bytes_processed: Math.floor((progress / 156) * 1024 * 1024 * 170),
                current_speed: 10 + Math.random() * 10,
                eta: Math.max(0, Math.floor((156 - progress) / 12)),
              }
            } : null);
          }, 1000);
        }
        setIsLoading(false);
      }
    };

    pollSession();
  }, [sessionId, navigate]);

  const cancelExecution = async () => {
    if (!sessionId || !canCancel) return;

    try {
      await invoke('cancel_execution', { sessionId });
      setCanCancel(false);
    } catch (error) {
      console.error('Tauri API not available, using demo mode:', error);
      // Demo mode - simulate cancellation
      setSession(prev => prev ? { ...prev, status: 'Cancelled' } : null);
      setCanCancel(false);
      alert('デモモード: 実行を停止しました');
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

  const formatDuration = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    } else {
      return `${minutes}:${secs.toString().padStart(2, '0')}`;
    }
  };

  const calculateProgress = (progress?: Progress) => {
    if (!progress || progress.total_ops === 0) return 0;
    return (progress.completed_ops / progress.total_ops) * 100;
  };

  if (!session && isLoading) {
    return (
      <div className="flex items-center justify-center min-h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">実行セッションを読み込んでいます...</p>
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

  if (session.status === 'Created') {
    return (
      <div className="text-center">
        <div className="animate-pulse">
          <div className="h-16 w-16 bg-gray-200 rounded-full mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">実行準備中</h2>
          <p className="text-gray-600">移動操作の準備をしています...</p>
        </div>
      </div>
    );
  }

  if (session.status === 'Failed') {
    return (
      <div className="space-y-6">
        <div className="text-center">
          <div className="w-16 h-16 bg-red-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-red-600 text-2xl">✗</span>
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">実行が失敗しました</h2>
          <p className="text-red-600 mb-4">{session.error}</p>
        </div>
        
        <div className="flex justify-center space-x-4">
          <button onClick={() => navigate('/')} className="btn-secondary">
            最初に戻る
          </button>
          {session.journal_path && (
            <button
              onClick={() => navigate(`/results/${sessionId}`)}
              className="btn-primary"
            >
              詳細を確認
            </button>
          )}
        </div>
      </div>
    );
  }

  if (session.status === 'Cancelled') {
    return (
      <div className="space-y-6">
        <div className="text-center">
          <div className="w-16 h-16 bg-yellow-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-yellow-600 text-2xl">⏸</span>
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">実行が停止されました</h2>
          <p className="text-gray-600 mb-4">移動操作がユーザーによって停止されました。</p>
        </div>
        
        <div className="flex justify-center space-x-4">
          <button onClick={() => navigate('/')} className="btn-secondary">
            最初に戻る
          </button>
          {session.journal_path && (
            <button
              onClick={() => navigate(`/results/${sessionId}`)}
              className="btn-primary"
            >
              実行結果を確認
            </button>
          )}
        </div>
      </div>
    );
  }

  if (session.status === 'Completed') {
    return (
      <div className="space-y-6">
        <div className="text-center">
          <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-green-600 text-2xl">✓</span>
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">実行が完了しました</h2>
          <p className="text-gray-600 mb-4">すべての移動操作が正常に完了しました。</p>
          <p className="text-sm text-gray-500">2秒後に結果画面に移動します...</p>
        </div>
        
        <div className="flex justify-center">
          <button
            onClick={() => navigate(`/results/${sessionId}`)}
            className="btn-primary"
          >
            結果を表示
          </button>
        </div>
      </div>
    );
  }

  // Running status
  const progress = session.progress;
  const progressPercent = calculateProgress(progress);

  return (
    <div className="space-y-6">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">フォルダ移動を実行中</h2>
        <p className="text-gray-600">移動操作を実行しています。しばらくお待ちください。</p>
      </div>

      {/* Progress Overview */}
      <div className="card">
        <div className="text-center mb-6">
          <div className="text-4xl font-bold text-primary-600 mb-2">
            {progressPercent.toFixed(1)}%
          </div>
          <div className="w-full bg-gray-200 rounded-full h-3">
            <div
              className="bg-primary-600 h-3 rounded-full transition-all duration-500 ease-out"
              style={{ width: `${progressPercent}%` }}
            ></div>
          </div>
        </div>
        
        {progress && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
            <div>
              <span className="font-medium text-gray-700">進行状況:</span>
              <span className="ml-2 text-gray-900">
                {progress.completed_ops.toLocaleString()} / {progress.total_ops.toLocaleString()} 操作
              </span>
            </div>
            <div>
              <span className="font-medium text-gray-700">処理済みサイズ:</span>
              <span className="ml-2 text-gray-900">
                {formatBytes(progress.bytes_processed)} 
                {progress.total_bytes && ` / ${formatBytes(progress.total_bytes)}`}
              </span>
            </div>
            {progress.current_speed && (
              <div>
                <span className="font-medium text-gray-700">処理速度:</span>
                <span className="ml-2 text-gray-900">
                  {progress.current_speed.toFixed(1)} ops/sec
                </span>
              </div>
            )}
            {progress.eta && (
              <div>
                <span className="font-medium text-gray-700">残り時間:</span>
                <span className="ml-2 text-gray-900">
                  約 {formatDuration(progress.eta)}
                </span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Current Operation */}
      {progress?.current_item && (
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-3">現在の操作</h3>
          <div className="bg-gray-50 rounded-md p-3">
            <p className="text-sm font-mono text-gray-900 break-all">
              {progress.current_item}
            </p>
          </div>
        </div>
      )}

      {/* Execution Details */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-3">実行情報</h3>
        <div className="space-y-2 text-sm">
          <div>
            <span className="font-medium text-gray-700">実行ID:</span>
            <span className="ml-2 text-gray-900 font-mono">{session.id}</span>
          </div>
          <div>
            <span className="font-medium text-gray-700">プランID:</span>
            <span className="ml-2 text-gray-900 font-mono">{session.plan_id}</span>
          </div>
          {session.journal_path && (
            <div>
              <span className="font-medium text-gray-700">ジャーナルファイル:</span>
              <span className="ml-2 text-gray-900 font-mono break-all">{session.journal_path}</span>
            </div>
          )}
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex justify-center space-x-4">
        {canCancel && (
          <button
            onClick={cancelExecution}
            className="btn-danger"
          >
            実行を停止
          </button>
        )}
        
        <button
          onClick={() => navigate('/')}
          className="btn-secondary"
          disabled={canCancel}
        >
          ホームに戻る
        </button>
      </div>

      {/* Warning Message */}
      <div className="card border-yellow-200 bg-yellow-50">
        <div className="flex items-start">
          <div className="flex-shrink-0">
            <span className="w-5 h-5 text-yellow-400">⚠</span>
          </div>
          <div className="ml-3">
            <h3 className="text-sm font-medium text-yellow-800">
              注意事項
            </h3>
            <div className="mt-1 text-sm text-yellow-700">
              <ul className="list-disc list-inside space-y-1">
                <li>実行中はコンピューターの電源を切らないでください</li>
                <li>大量のファイルの移動中は他の重い操作を避けてください</li>
                <li>ジャーナルファイルは移動操作の記録として保存されます</li>
                <li>問題が発生した場合、ジャーナルファイルから操作をアンドゥできます</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ExecutionPage;