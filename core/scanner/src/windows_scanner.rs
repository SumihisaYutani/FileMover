use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)] 
use std::os::windows::ffi::OsStringExt;
#[cfg(windows)]
use winapi::um::fileapi::*;
#[cfg(windows)]
use winapi::um::handleapi::*;
#[cfg(windows)]
use winapi::um::winbase::*;
use winapi::um::minwinbase::WIN32_FIND_DATAW;
#[cfg(windows)]
use winapi::um::winnt::*;
#[cfg(windows)]
use winapi::shared::winerror::*;
use tracing::{debug, warn, error};
use filemover_types::{ScanOptions, FileMoverError};
use crate::scanner::DirectoryEntry;

#[cfg(windows)]
pub struct WindowsDirectoryWalker {
    options: ScanOptions,
}

#[cfg(windows)]
impl WindowsDirectoryWalker {
    pub fn new(options: ScanOptions) -> Self {
        Self { options }
    }

    pub fn walk(&self, root: &Path) -> Result<Vec<DirectoryEntry>, FileMoverError> {
        let mut entries = Vec::new();
        self.walk_recursive(root, 0, &mut entries)?;
        Ok(entries)
    }

    fn walk_recursive(&self, dir: &Path, depth: u32, entries: &mut Vec<DirectoryEntry>) -> Result<(), FileMoverError> {
        // 最大深度チェック
        if let Some(max_depth) = self.options.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }

        debug!("Walking Windows directory at depth {}: {}", depth, dir.display());

        // 長パス対応のためUNCパス形式に変換
        let search_path = self.to_long_path(dir)?;
        let pattern = format!("{}\\*", search_path);
        
        // UTF-16に変換
        let pattern_wide: Vec<u16> = pattern.encode_utf16().chain(std::iter::once(0)).collect();

        let mut find_data = WIN32_FIND_DATAW {
            dwFileAttributes: 0,
            ftCreationTime: unsafe { std::mem::zeroed() },
            ftLastAccessTime: unsafe { std::mem::zeroed() },
            ftLastWriteTime: unsafe { std::mem::zeroed() },
            nFileSizeHigh: 0,
            nFileSizeLow: 0,
            dwReserved0: 0,
            dwReserved1: 0,
            cFileName: [0; 260],
            cAlternateFileName: [0; 14],
        };

        let handle = unsafe {
            FindFirstFileW(pattern_wide.as_ptr(), &mut find_data)
        };

        if handle == INVALID_HANDLE_VALUE {
            let error = unsafe { winapi::um::errhandlingapi::GetLastError() };
            match error {
                ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => {
                    debug!("No files found in directory: {}", dir.display());
                    return Ok(());
                }
                ERROR_ACCESS_DENIED => {
                    warn!("Access denied to directory: {}", dir.display());
                    // アクセス拒否でもディレクトリエントリとして記録
                    entries.push(DirectoryEntry {
                        path: dir.to_path_buf(),
                        is_directory: true,
                        is_junction: false,
                        access_denied: true,
                        size_bytes: None,
                    });
                    return Ok(());
                }
                _ => {
                    return Err(FileMoverError::Scan {
                        path: dir.to_path_buf(),
                        message: format!("FindFirstFile failed with error: {}", error),
                    });
                }
            }
        }

        loop {
            let filename = self.wide_string_to_path(&find_data.cFileName)?;
            
            // "." と ".." をスキップ
            if filename == "." || filename == ".." {
                if unsafe { FindNextFileW(handle, &mut find_data) } == 0 {
                    break;
                }
                continue;
            }

            let full_path = dir.join(&filename);
            let is_directory = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            let is_junction = (find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
            
            // ディレクトリの場合のみ処理
            if is_directory {
                // 除外パスのチェック
                if self.is_excluded_path(&full_path) {
                    if unsafe { FindNextFileW(handle, &mut find_data) } == 0 {
                        break;
                    }
                    continue;
                }

                // サイズ計算（オプション）
                let size_bytes = if self.should_calculate_size() {
                    self.calculate_directory_size(&full_path).ok()
                } else {
                    None
                };

                let entry = DirectoryEntry {
                    path: full_path.clone(),
                    is_directory: true,
                    is_junction,
                    access_denied: false,
                    size_bytes,
                };

                entries.push(entry);

                // ジャンクション/シンボリックリンクの追跡オプション
                if is_junction && !self.options.follow_junctions {
                    debug!("Skipping junction: {}", full_path.display());
                } else {
                    // 再帰的にサブディレクトリを走査
                    if let Err(e) = self.walk_recursive(&full_path, depth + 1, entries) {
                        warn!("Failed to walk subdirectory {}: {}", full_path.display(), e);
                        // エラーがあっても他のディレクトリの処理を継続
                    }
                }
            }

            if unsafe { FindNextFileW(handle, &mut find_data) } == 0 {
                break;
            }
        }

        unsafe { FindClose(handle) };
        Ok(())
    }

    fn to_long_path(&self, path: &Path) -> Result<String, FileMoverError> {
        let path_str = path.to_string_lossy();
        
        // すでに長パス形式の場合はそのまま返す
        if path_str.starts_with("\\\\?\\") {
            return Ok(path_str.into_owned());
        }

        // UNCパスの場合
        if path_str.starts_with("\\\\") {
            return Ok(format!("\\\\?\\UNC\\{}", &path_str[2..]));
        }

        // 通常のパスの場合
        if path.is_absolute() {
            Ok(format!("\\\\?\\{}", path_str))
        } else {
            // 相対パスを絶対パスに変換してから長パス形式に
            let abs_path = path.canonicalize()
                .map_err(|e| FileMoverError::Scan {
                    path: path.to_path_buf(),
                    message: format!("Failed to canonicalize path: {}", e),
                })?;
            Ok(format!("\\\\?\\{}", abs_path.display()))
        }
    }

    fn wide_string_to_path(&self, wide_str: &[u16]) -> Result<String, FileMoverError> {
        let null_pos = wide_str.iter().position(|&c| c == 0).unwrap_or(wide_str.len());
        let os_string = OsString::from_wide(&wide_str[..null_pos]);
        
        os_string.into_string().map_err(|_| FileMoverError::Pattern {
            message: "Invalid filename encoding".to_string(),
        })
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        for excluded in &self.options.excluded_paths {
            if path.starts_with(excluded) {
                return true;
            }
        }
        false
    }

    fn should_calculate_size(&self) -> bool {
        // 大量のディレクトリがある場合はサイズ計算をスキップしてパフォーマンスを優先
        false
    }

    fn calculate_directory_size(&self, _dir: &Path) -> Result<u64, FileMoverError> {
        // 実装は複雑になるため、必要に応じて後で実装
        // 現在は簡単なスタブ
        Ok(0)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_long_path_conversion() {
        let walker = WindowsDirectoryWalker::new(ScanOptions::default());
        
        // 通常のパス
        let path = Path::new("C:\\Users\\Test");
        let long_path = walker.to_long_path(path).unwrap();
        assert_eq!(long_path, "\\\\?\\C:\\Users\\Test");
        
        // UNCパス
        let unc_path = Path::new("\\\\server\\share\\folder");
        let long_unc = walker.to_long_path(unc_path).unwrap();
        assert_eq!(long_unc, "\\\\?\\UNC\\server\\share\\folder");
    }

    #[test]
    fn test_wide_string_conversion() {
        let walker = WindowsDirectoryWalker::new(ScanOptions::default());
        
        let test_str = "test_folder";
        let wide: Vec<u16> = test_str.encode_utf16().chain(std::iter::once(0)).collect();
        let converted = walker.wide_string_to_path(&wide).unwrap();
        
        assert_eq!(converted, test_str);
    }
}