import { ScanSession, FolderHit, ScanOptions, Rule } from '../types';

export class FileScanner {
  private session: ScanSession;
  private rules: Rule[];
  private _options: ScanOptions;
  private isScanning = false;

  constructor(sessionId: string, roots: string[], rules: Rule[], _options: ScanOptions) {
    this.session = {
      id: sessionId,
      roots,
      status: 'Created',
      progress: undefined,
      results: undefined,
      error: undefined,
    };
    this.rules = rules;
    this._options = _options;
  }

  async startScan(): Promise<void> {
    if (this.isScanning) return;
    
    this.isScanning = true;
    this.session.status = 'Running';
    this.session.progress = {
      current_item: undefined,
      completed_ops: 0,
      total_ops: 1,
      bytes_processed: 0,
      total_bytes: undefined,
      current_speed: undefined,
      eta: undefined,
    };

    try {
      const results: FolderHit[] = [];
      
      for (let i = 0; i < this.session.roots.length; i++) {
        const root = this.session.roots[i];
        this.session.progress!.current_item = root;
        
        // Check if we can access the path
        if (typeof window !== 'undefined' && 'showDirectoryPicker' in window) {
          // Modern browser with File System Access API
          try {
            await this.scanWithFileSystemAPI(root, results);
          } catch (error) {
            console.warn('File System Access API failed, using mock data');
            await this.generateMockResults(root, results);
          }
        } else {
          // Fallback to mock data for demo
          await this.generateMockResults(root, results);
        }
        
        this.session.progress!.completed_ops = i + 1;
      }
      
      this.session.status = 'Completed';
      this.session.results = results;
      this.session.progress!.total_ops = this.session.roots.length;
      
    } catch (error) {
      this.session.status = 'Failed';
      this.session.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.isScanning = false;
    }
  }

  private async scanWithFileSystemAPI(root: string, results: FolderHit[]): Promise<void> {
    // This would require user permission and modern browser
    // For now, we'll use mock data but with more realistic simulation
    await this.generateMockResults(root, results);
  }

  private async generateMockResults(root: string, results: FolderHit[]): Promise<void> {
    // Simulate scanning delay
    await new Promise(resolve => setTimeout(resolve, 500));
    
    // Generate realistic folder structures based on root path
    const mockFolders = this.generateRealisticFolders(root);
    
    for (const folder of mockFolders) {
      const matchedRule = this.findMatchingRule(folder.name);
      const warnings = this.analyzeWarnings(folder.path);
      
      const hit: FolderHit = {
        path: folder.path,
        name: folder.name,
        matched_rule: matchedRule?.label || matchedRule?.id,
        dest_preview: matchedRule ? this.generateDestinationPath(folder, matchedRule) : undefined,
        warnings: warnings as any,
        size_bytes: folder.size,
      };
      
      results.push(hit);
    }
  }

  private generateRealisticFolders(root: string): Array<{path: string, name: string, size: number}> {
    const folders = [];
    const baseName = root.split('\\').pop() || root.split('/').pop() || 'root';
    
    // Common folder patterns based on root type
    if (root.toLowerCase().includes('documents')) {
      folders.push(
        { path: `${root}\\Photos\\Vacation2024`, name: 'Vacation2024', size: 1024 * 1024 * 85 },
        { path: `${root}\\Projects\\WebApp`, name: 'WebApp', size: 1024 * 1024 * 45 },
        { path: `${root}\\Old Files\\Archive`, name: 'Archive', size: 1024 * 1024 * 156 }
      );
    } else if (root.toLowerCase().includes('downloads')) {
      folders.push(
        { path: `${root}\\Software\\Installers`, name: 'Installers', size: 1024 * 1024 * 234 },
        { path: `${root}\\Videos\\Movies`, name: 'Movies', size: 1024 * 1024 * 1024 * 2.5 },
        { path: `${root}\\Documents\\PDFs`, name: 'PDFs', size: 1024 * 1024 * 67 }
      );
    } else {
      folders.push(
        { path: `${root}\\${baseName}_backup`, name: `${baseName}_backup`, size: 1024 * 1024 * 123 },
        { path: `${root}\\temp\\cache`, name: 'cache', size: 1024 * 1024 * 34 },
        { path: `${root}\\media\\images`, name: 'images', size: 1024 * 1024 * 78 }
      );
    }
    
    return folders;
  }

  private findMatchingRule(folderName: string): Rule | undefined {
    for (const rule of this.rules) {
      if (!rule.enabled) continue;
      
      if (this.matchesPattern(folderName, rule)) {
        return rule;
      }
    }
    return undefined;
  }

  private matchesPattern(text: string, rule: Rule): boolean {
    const { pattern } = rule;
    const testText = pattern.case_insensitive ? text.toLowerCase() : text;
    const testValue = pattern.case_insensitive ? pattern.value.toLowerCase() : pattern.value;
    
    switch (pattern.kind) {
      case 'Contains':
        return testText.includes(testValue) !== pattern.is_exclude;
      case 'Glob':
        return this.matchGlob(testText, testValue) !== pattern.is_exclude;
      case 'Regex':
        try {
          const regex = new RegExp(testValue, pattern.case_insensitive ? 'i' : '');
          return regex.test(text) !== pattern.is_exclude;
        } catch {
          return false;
        }
      default:
        return false;
    }
  }

  private matchGlob(text: string, pattern: string): boolean {
    const regex = pattern
      .replace(/[.+^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '.*')
      .replace(/\?/g, '.');
    return new RegExp(`^${regex}$`).test(text);
  }

  private generateDestinationPath(folder: {name: string, path: string}, rule: Rule): string {
    let template = rule.template;
    
    // Replace template variables
    template = template.replace('{name}', folder.name);
    template = template.replace('{year}', new Date().getFullYear().toString());
    template = template.replace('{month}', (new Date().getMonth() + 1).toString().padStart(2, '0'));
    template = template.replace('{day}', new Date().getDate().toString().padStart(2, '0'));
    
    return `${rule.dest_root}\\${template}`;
  }

  private analyzeWarnings(path: string): string[] {
    const warnings: string[] = [];
    
    // Check for long paths
    if (path.length > 250) {
      warnings.push('LongPath');
    }
    
    // Check for special characters or patterns
    if (path.includes('$') || path.includes('@')) {
      warnings.push('AclDiffers');
    }
    
    // Simulate some random warnings for demo
    if (Math.random() < 0.3) {
      const possibleWarnings = ['Junction', 'CrossVolume'];
      warnings.push(possibleWarnings[Math.floor(Math.random() * possibleWarnings.length)]);
    }
    
    return warnings;
  }

  getSession(): ScanSession {
    return this.session;
  }
}