import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { open } from '@tauri-apps/api/dialog';
import { ExecutionSession, UndoResult, JournalValidation } from '../types';

function ResultsPage() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<ExecutionSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [journalValidation, setJournalValidation] = useState<JournalValidation | null>(null);
  const [undoResult, setUndoResult] = useState<UndoResult | null>(null);
  const [isUndoing, setIsUndoing] = useState(false);

  useEffect(() => {
    if (!sessionId) return;

    const loadData = async () => {
      try {
        const sessionData = await invoke<ExecutionSession>('get_execution_session', { sessionId });
        setSession(sessionData);
        
        // Load journal validation if available
        if (sessionData.journal_path) {
          try {
            const validation = await invoke<JournalValidation>('validate_journal', {
              journalPath: sessionData.journal_path,
            });
            setJournalValidation(validation);
          } catch (error) {
            console.error('Failed to validate journal:', error);
          }
        }
      } catch (error) {
        console.error('Tauri API not available, using demo mode:', error);
        
        // Demo mode - create mock execution session results
        if (sessionId?.startsWith('execution-demo-')) {
          const mockExecutionSession: ExecutionSession = {
            id: sessionId,
            plan_id: 'plan-demo-123',
            status: 'Completed',
            progress: {
              current_item: undefined,
              completed_ops: 156,
              total_ops: 156,
              bytes_processed: 1024 * 1024 * 170, // 170MB
              total_bytes: 1024 * 1024 * 170,
              current_speed: undefined,
              eta: undefined,
            },
            journal_path: 'C:\\Users\\Demo\\AppData\\Local\\FileMover\\execution_journal.json',
            error: undefined,
          };
          setSession(mockExecutionSession);
          
          // Mock journal validation
          const mockJournalValidation: JournalValidation = {
            is_valid: true,
            total_entries: 156,
            successful_entries: 154,
            failed_entries: 2,
            skipped_entries: 0,
            undoable_entries: 154,
            issues: [
              'ファイル IMG_045.jpg の移動に失敗しました（アクセス拒否）',
              'ファイル project.config の移動に失敗しました（使用中）'
            ],
          };
          setJournalValidation(mockJournalValidation);
        }
      } finally {
        setIsLoading(false);
      }
    };

    loadData();
  }, [sessionId]);

  const exportJournal = async () => {
    if (!session?.journal_path) return;

    try {
      const savePath = await open({
        directory: false,
        multiple: false,
        defaultPath: 'filemover-journal.json',
        filters: [{
          name: 'JSON Files',
          extensions: ['json']
        }]
      });

      if (savePath && typeof savePath === 'string') {
        await invoke('export_journal', {
          journalPath: session.journal_path,
          outputPath: savePath,
        });
        alert('ジャーナルファイルをエクスポートしました。');
      }
    } catch (error) {
      console.error('Tauri API not available, using demo mode:', error);
      // Demo mode - simulate export
      const demoPath = prompt('デモ用: エクスポート先パスを入力してください', 'C:\\Users\\Demo\\Desktop\\filemover-journal.json');
      if (demoPath) {
        alert(`デモモード: ジャーナルファイルを ${demoPath} にエクスポートしました。`);
      }
    }
  };

  const undoOperations = async () => {
    if (!session?.journal_path || !journalValidation?.is_valid || isUndoing) return;

    const confirmed = confirm(
      `${journalValidation.undoable_entries}個の操作をアンドゥします。この操作は元に戻せません。続行しますか？`
    );

    if (!confirmed) return;

    try {
      setIsUndoing(true);
      const result = await invoke<UndoResult>('undo_operations', {
        journalPath: session.journal_path,
      });
      setUndoResult(result);
      
      // Refresh journal validation after undo
      const validation = await invoke<JournalValidation>('validate_journal', {
        journalPath: session.journal_path,
      });
      setJournalValidation(validation);
    } catch (error) {
      console.error('Tauri API not available, using demo mode:', error);
      // Demo mode - simulate undo
      const mockUndoResult: UndoResult = {
        total_operations: 154,
        undone_operations: 152,
        failed_operations: 2,
        skipped_operations: 0,
        errors: [
          'ファイル IMG_045.jpg のアンドゥに失敗: 移動先ファイルが見つかりません',
          'ファイル project.config のアンドゥに失敗: アクセス拒否'
        ],
      };
      setUndoResult(mockUndoResult);
      
      // Update journal validation after undo
      setJournalValidation(prev => prev ? {
        ...prev,
        undoable_entries: 0,
        issues: [...prev.issues, 'アンドゥ操作が完了しました']
      } : null);
      
      alert('デモモード: アンドゥ操作が完了しました。152/154件の操作を元に戻しました。');
    } finally {
      setIsUndoing(false);
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

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Completed':
        return <span className="text-green-600 text-2xl">✓</span>;
      case 'Failed':
        return <span className="text-red-600 text-2xl">✗</span>;
      case 'Cancelled':
        return <span className="text-yellow-600 text-2xl">⏸</span>;
      default:
        return <span className="text-gray-600 text-2xl">?</span>;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Completed':
        return 'text-green-600';
      case 'Failed':
        return 'text-red-600';
      case 'Cancelled':
        return 'text-yellow-600';
      default:
        return 'text-gray-600';
    }
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'Completed':
        return '正常完了';
      case 'Failed':
        return '実行失敗';
      case 'Cancelled':
        return 'ユーザー停止';
      default:
        return status;
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">実行結果を読み込んでいます...</p>
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

  const progress = session.progress;
  const hasJournal = !!session.journal_path;
  const canUndo = journalValidation?.is_valid && journalValidation.undoable_entries > 0;

  return (
    <div className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
          {getStatusIcon(session.status)}
        </div>
        <h2 className="text-2xl font-bold text-gray-900 mb-2">実行結果</h2>
        <p className={`text-lg font-medium ${getStatusColor(session.status)}`}>
          {getStatusLabel(session.status)}
        </p>
        {session.error && (
          <p className="text-red-600 mt-2 text-sm">{session.error}</p>
        )}
      </div>

      {/* Execution Summary */}
      {progress && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="card">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">処理操作数</h3>
            <p className="text-3xl font-bold text-blue-600">
              {progress.completed_ops.toLocaleString()}
            </p>
            <p className="text-sm text-gray-500 mt-1">
              / {progress.total_ops.toLocaleString()} 操作
            </p>
          </div>
          
          <div className="card">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">処理済みサイズ</h3>
            <p className="text-3xl font-bold text-green-600">
              {formatBytes(progress.bytes_processed)}
            </p>
            {progress.total_bytes && (
              <p className="text-sm text-gray-500 mt-1">
                / {formatBytes(progress.total_bytes)}
              </p>
            )}
          </div>
          
          <div className="card">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">完了率</h3>
            <p className="text-3xl font-bold text-purple-600">
              {progress.total_ops > 0 
                ? ((progress.completed_ops / progress.total_ops) * 100).toFixed(1)
                : '0'
              }%
            </p>
            <p className="text-sm text-gray-500 mt-1">操作完了</p>
          </div>
        </div>
      )}

      {/* Session Details */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">実行詳細</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
          <div>
            <span className="font-medium text-gray-700">実行ID:</span>
            <span className="ml-2 text-gray-900 font-mono">{session.id}</span>
          </div>
          <div>
            <span className="font-medium text-gray-700">プランID:</span>
            <span className="ml-2 text-gray-900 font-mono">{session.plan_id}</span>
          </div>
          <div>
            <span className="font-medium text-gray-700">ステータス:</span>
            <span className={`ml-2 font-medium ${getStatusColor(session.status)}`}>
              {getStatusLabel(session.status)}
            </span>
          </div>
          {session.journal_path && (
            <div className="md:col-span-2">
              <span className="font-medium text-gray-700">ジャーナルファイル:</span>
              <span className="ml-2 text-gray-900 font-mono break-all">{session.journal_path}</span>
            </div>
          )}
        </div>
      </div>

      {/* Journal Validation */}
      {journalValidation && (
        <div className="card">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">ジャーナル情報</h3>
          
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
            <div className="text-center">
              <p className="text-2xl font-bold text-blue-600">{journalValidation.total_entries}</p>
              <p className="text-sm text-gray-500">総エントリ数</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-green-600">{journalValidation.successful_entries}</p>
              <p className="text-sm text-gray-500">成功</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-red-600">{journalValidation.failed_entries}</p>
              <p className="text-sm text-gray-500">失敗</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-purple-600">{journalValidation.undoable_entries}</p>
              <p className="text-sm text-gray-500">アンドゥ可能</p>
            </div>
          </div>
          
          {journalValidation.issues.length > 0 && (
            <div className="mt-4">
              <h4 className="font-medium text-gray-900 mb-2">ジャーナルの問題:</h4>
              <ul className="list-disc list-inside space-y-1 text-sm text-red-600">
                {journalValidation.issues.map((issue, index) => (
                  <li key={index}>{issue}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Undo Result */}
      {undoResult && (
        <div className="card border-blue-200 bg-blue-50">
          <h3 className="text-lg font-semibold text-blue-900 mb-4">アンドゥ結果</h3>
          
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
            <div className="text-center">
              <p className="text-2xl font-bold text-blue-600">{undoResult.total_operations}</p>
              <p className="text-sm text-blue-700">総操作数</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-green-600">{undoResult.undone_operations}</p>
              <p className="text-sm text-blue-700">アンドゥ成功</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-red-600">{undoResult.failed_operations}</p>
              <p className="text-sm text-blue-700">アンドゥ失敗</p>
            </div>
            <div className="text-center">
              <p className="text-2xl font-bold text-gray-600">{undoResult.skipped_operations}</p>
              <p className="text-sm text-blue-700">スキップ</p>
            </div>
          </div>
          
          {undoResult.errors.length > 0 && (
            <div className="mt-4">
              <h4 className="font-medium text-blue-900 mb-2">アンドゥエラー:</h4>
              <ul className="list-disc list-inside space-y-1 text-sm text-red-700">
                {undoResult.errors.map((error, index) => (
                  <li key={index}>{error}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Action Buttons */}
      <div className="flex flex-wrap justify-center gap-4">
        <button
          onClick={() => navigate('/')}
          className="btn-primary"
        >
          新しいスキャンを開始
        </button>
        
        {hasJournal && (
          <button
            onClick={exportJournal}
            className="btn-secondary"
          >
            ジャーナルをエクスポート
          </button>
        )}
        
        {canUndo && (
          <button
            onClick={undoOperations}
            disabled={isUndoing}
            className="btn-danger"
          >
            {isUndoing ? 'アンドゥ中...' : `操作をアンドゥ (${journalValidation?.undoable_entries}件)`}
          </button>
        )}
      </div>

      {/* Important Notes */}
      {hasJournal && (
        <div className="card border-blue-200 bg-blue-50">
          <div className="flex items-start">
            <div className="flex-shrink-0">
              <span className="w-5 h-5 text-blue-400">ℹ</span>
            </div>
            <div className="ml-3">
              <h3 className="text-sm font-medium text-blue-800">
                ジャーナルファイルについて
              </h3>
              <div className="mt-1 text-sm text-blue-700">
                <ul className="list-disc list-inside space-y-1">
                  <li>ジャーナルファイルには実行されたすべての操作が記録されています</li>
                  <li>アンドゥ機能を使用して操作を元に戻すことができます</li>
                  <li>ジャーナルファイルをエクスポートして安全な場所に保存することを推奨します</li>
                  <li>アンドゥは一度に全体を実行し、部分的なアンドゥはできません</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default ResultsPage;