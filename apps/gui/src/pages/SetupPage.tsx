import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import { open } from '@tauri-apps/api/dialog';
import { Config, Rule, PatternKind, ConflictPolicy } from '../types';
import { sessionManager } from '../services/sessionManager';

function SetupPage() {
  const navigate = useNavigate();
  const [config, setConfig] = useState<Config>({
    roots: [],
    rules: [],
    options: {
      normalization: {
        normalize_unicode: true,
        normalize_width: true,
        strip_diacritics: false,
        normalize_case: false,
      },
      follow_junctions: false,
      system_protections: true,
      excluded_paths: [],
    },
    profiles: [],
  });
  
  const [isLoading, setIsLoading] = useState(false);

  const addRoot = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected && typeof selected === 'string') {
        setConfig(prev => ({
          ...prev,
          roots: [...prev.roots, selected],
        }));
      }
    } catch (error) {
      // Browser environment - use File System Access API or fallback
      if ('showDirectoryPicker' in window) {
        try {
          const dirHandle = await (window as any).showDirectoryPicker();
          const path = dirHandle.name; // Browser gives us directory name, not full path
          const fullPath = `[Browser] ${path}`;
          setConfig(prev => ({
            ...prev,
            roots: [...prev.roots, fullPath],
          }));
          return;
        } catch (fsError) {
          console.log('User cancelled directory picker or not supported');
        }
      }
      
      // Final fallback - common directory options
      const commonPaths = [
        'C:\\Users\\Documents',
        'C:\\Users\\Downloads', 
        'C:\\Users\\Pictures',
        'C:\\Users\\Desktop',
        'D:\\MyFiles',
        '入力で指定...'
      ];
      
      const selectedIndex = prompt(
        `フォルダを選択してください:\n${commonPaths.map((p, i) => `${i}: ${p}`).join('\n')}`,
        '0'
      );
      
      if (selectedIndex !== null) {
        const index = parseInt(selectedIndex);
        if (index >= 0 && index < commonPaths.length - 1) {
          setConfig(prev => ({
            ...prev,
            roots: [...prev.roots, commonPaths[index]],
          }));
        } else if (index === commonPaths.length - 1) {
          const customPath = prompt('フォルダパスを入力してください:', 'C:\\Users\\YourName\\Documents');
          if (customPath) {
            setConfig(prev => ({
              ...prev,
              roots: [...prev.roots, customPath],
            }));
          }
        }
      }
    }
  };

  const removeRoot = (index: number) => {
    setConfig(prev => ({
      ...prev,
      roots: prev.roots.filter((_, i) => i !== index),
    }));
  };

  const addRule = () => {
    const newRule: Rule = {
      id: crypto.randomUUID(),
      enabled: true,
      pattern: {
        kind: 'Glob' as PatternKind,
        value: '*',
        is_exclude: false,
        case_insensitive: true,
      },
      dest_root: '',
      template: '{name}',
      policy: 'AutoRename' as ConflictPolicy,
      priority: config.rules.length,
    };
    
    setConfig(prev => ({
      ...prev,
      rules: [...prev.rules, newRule],
    }));
  };

  const updateRule = (index: number, updates: Partial<Rule>) => {
    setConfig(prev => ({
      ...prev,
      rules: prev.rules.map((rule, i) => 
        i === index ? { ...rule, ...updates } : rule
      ),
    }));
  };

  const removeRule = (index: number) => {
    setConfig(prev => ({
      ...prev,
      rules: prev.rules.filter((_, i) => i !== index),
    }));
  };

  const selectDestination = async (index: number) => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected && typeof selected === 'string') {
        updateRule(index, { dest_root: selected });
      }
    } catch (error) {
      // Browser environment - use File System Access API or fallback
      if ('showDirectoryPicker' in window) {
        try {
          const dirHandle = await (window as any).showDirectoryPicker();
          const path = dirHandle.name;
          const fullPath = `[Browser] ${path}`;
          updateRule(index, { dest_root: fullPath });
          return;
        } catch (fsError) {
          console.log('User cancelled directory picker or not supported');
        }
      }
      
      // Fallback - common destination options
      const commonDests = [
        'C:\\Users\\Desktop\\FileMover',
        'C:\\Users\\Documents\\Organized',
        'C:\\Archive',
        'D:\\Sorted',
        'C:\\Users\\Pictures\\Organized',
        '入力で指定...'
      ];
      
      const selectedIndex = prompt(
        `移動先フォルダを選択してください:\n${commonDests.map((p, i) => `${i}: ${p}`).join('\n')}`,
        '0'
      );
      
      if (selectedIndex !== null) {
        const index = parseInt(selectedIndex);
        if (index >= 0 && index < commonDests.length - 1) {
          updateRule(index, { dest_root: commonDests[index] });
        } else if (index === commonDests.length - 1) {
          const customPath = prompt('移動先パスを入力してください:', 'C:\\Users\\Desktop\\FileMover');
          if (customPath) {
            updateRule(index, { dest_root: customPath });
          }
        }
      }
    }
  };

  const startScan = async () => {
    console.log('startScan called, config.roots:', config.roots);
    console.log('config.roots.length:', config.roots.length);
    
    if (config.roots.length === 0) {
      alert('スキャン対象のルートフォルダを選択してください。');
      return;
    }

    setIsLoading(true);
    try {
      // Try Tauri API first
      const sessionId = await invoke<string>('create_scan_session', {
        roots: config.roots,
        options: config.options,
      });
      
      await invoke('start_scan', { sessionId });
      navigate(`/scan-results/${sessionId}`);
    } catch (error) {
      console.error('Tauri API not available, using real browser implementation:', error);
      
      try {
        // Use real browser-based implementation
        const sessionId = sessionManager.createScanSession(config.roots, config.options);
        
        // Start scanning in background
        sessionManager.startScan(sessionId, config.rules, config.options)
          .catch(err => console.error('Scan failed:', err));
        
        navigate(`/scan-results/${sessionId}`);
      } catch (browserError) {
        console.error('Browser implementation failed:', browserError);
        // Final fallback to demo mode
        const demoSessionId = `demo-${Date.now()}`;
        alert(`フォールバック: デモモードで動作します (セッションID: ${demoSessionId})`);
        navigate(`/scan-results/${demoSessionId}`);
      }
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-6">フォルダスキャン設定</h2>
      </div>

      {/* Root Folders Section */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">スキャン対象フォルダ</h3>
        
        <div className="space-y-3">
          {config.roots.map((root, index) => (
            <div key={index} className="flex items-center justify-between p-3 bg-gray-50 rounded-md">
              <span className="text-sm font-mono text-gray-700">{root}</span>
              <button
                onClick={() => removeRoot(index)}
                className="text-red-600 hover:text-red-800 text-sm"
              >
                削除
              </button>
            </div>
          ))}
          
          {config.roots.length === 0 && (
            <p className="text-gray-500 text-sm">スキャン対象のフォルダが選択されていません。</p>
          )}
        </div>
        
        <button
          onClick={addRoot}
          className="btn-secondary mt-4"
        >
          フォルダを追加
        </button>
      </div>

      {/* Rules Section */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">移動ルール</h3>
        
        <div className="space-y-4">
          {config.rules.map((rule, index) => (
            <div key={rule.id} className="border border-gray-200 rounded-md p-4 space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => updateRule(index, { enabled: e.target.checked })}
                    className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                  />
                  <input
                    type="text"
                    value={rule.label || ''}
                    onChange={(e) => updateRule(index, { label: e.target.value })}
                    placeholder="ルール名"
                    className="input flex-1"
                  />
                </div>
                <button
                  onClick={() => removeRule(index)}
                  className="text-red-600 hover:text-red-800 text-sm"
                >
                  削除
                </button>
              </div>
              
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    パターン種別
                  </label>
                  <select
                    value={rule.pattern.kind}
                    onChange={(e) => updateRule(index, {
                      pattern: { ...rule.pattern, kind: e.target.value as PatternKind }
                    })}
                    className="input"
                  >
                    <option value="Glob">Glob</option>
                    <option value="Regex">正規表現</option>
                    <option value="Contains">部分一致</option>
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    パターン
                  </label>
                  <input
                    type="text"
                    value={rule.pattern.value}
                    onChange={(e) => updateRule(index, {
                      pattern: { ...rule.pattern, value: e.target.value }
                    })}
                    className="input"
                    placeholder="*.jpg"
                  />
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    競合処理
                  </label>
                  <select
                    value={rule.policy}
                    onChange={(e) => updateRule(index, { policy: e.target.value as ConflictPolicy })}
                    className="input"
                  >
                    <option value="AutoRename">自動リネーム</option>
                    <option value="Skip">スキップ</option>
                    <option value="Overwrite">上書き</option>
                  </select>
                </div>
              </div>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    移動先フォルダ
                  </label>
                  <div className="flex space-x-2">
                    <input
                      type="text"
                      value={rule.dest_root}
                      onChange={(e) => updateRule(index, { dest_root: e.target.value })}
                      className="input flex-1"
                      placeholder="移動先を選択"
                    />
                    <button
                      onClick={() => selectDestination(index)}
                      className="btn-secondary"
                    >
                      選択
                    </button>
                  </div>
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    ファイル名テンプレート
                  </label>
                  <input
                    type="text"
                    value={rule.template}
                    onChange={(e) => updateRule(index, { template: e.target.value })}
                    className="input"
                    placeholder="{name}"
                  />
                </div>
              </div>
              
              <div className="flex items-center space-x-4">
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={rule.pattern.case_insensitive}
                    onChange={(e) => updateRule(index, {
                      pattern: { ...rule.pattern, case_insensitive: e.target.checked }
                    })}
                    className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                  />
                  <span className="ml-2 text-sm text-gray-700">大文字小文字を区別しない</span>
                </label>
                
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={rule.pattern.is_exclude}
                    onChange={(e) => updateRule(index, {
                      pattern: { ...rule.pattern, is_exclude: e.target.checked }
                    })}
                    className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                  />
                  <span className="ml-2 text-sm text-gray-700">除外パターン</span>
                </label>
              </div>
            </div>
          ))}
          
          {config.rules.length === 0 && (
            <p className="text-gray-500 text-sm">移動ルールが設定されていません。</p>
          )}
        </div>
        
        <button
          onClick={addRule}
          className="btn-secondary mt-4"
        >
          ルールを追加
        </button>
      </div>

      {/* Scan Options */}
      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">スキャンオプション</h3>
        
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-3">
              <h4 className="font-medium text-gray-900">テキスト正規化</h4>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.normalization.normalize_unicode}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      normalization: {
                        ...prev.options.normalization,
                        normalize_unicode: e.target.checked,
                      },
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">Unicode正規化</span>
              </label>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.normalization.normalize_width}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      normalization: {
                        ...prev.options.normalization,
                        normalize_width: e.target.checked,
                      },
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">文字幅正規化</span>
              </label>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.normalization.strip_diacritics}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      normalization: {
                        ...prev.options.normalization,
                        strip_diacritics: e.target.checked,
                      },
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">発音区別記号除去</span>
              </label>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.normalization.normalize_case}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      normalization: {
                        ...prev.options.normalization,
                        normalize_case: e.target.checked,
                      },
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">大文字小文字正規化</span>
              </label>
            </div>
            
            <div className="space-y-3">
              <h4 className="font-medium text-gray-900">スキャン設定</h4>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.follow_junctions}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      follow_junctions: e.target.checked,
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">ジャンクションを辿る</span>
              </label>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={config.options.system_protections}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      system_protections: e.target.checked,
                    },
                  }))}
                  className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="ml-2 text-sm text-gray-700">システム保護</span>
              </label>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  最大深度
                </label>
                <input
                  type="number"
                  value={config.options.max_depth || ''}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      max_depth: e.target.value ? parseInt(e.target.value) : undefined,
                    },
                  }))}
                  className="input w-32"
                  placeholder="無制限"
                  min="1"
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  並列スレッド数
                </label>
                <input
                  type="number"
                  value={config.options.parallel_threads || ''}
                  onChange={(e) => setConfig(prev => ({
                    ...prev,
                    options: {
                      ...prev.options,
                      parallel_threads: e.target.value ? parseInt(e.target.value) : undefined,
                    },
                  }))}
                  className="input w-32"
                  placeholder="自動"
                  min="1"
                  max="16"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex justify-end space-x-4">
        <button
          onClick={startScan}
          disabled={isLoading || config.roots.length === 0}
          className="btn-primary btn-lg"
        >
          {isLoading ? 'スキャン中...' : 'スキャン開始'}
        </button>
      </div>
    </div>
  );
}

export default SetupPage;