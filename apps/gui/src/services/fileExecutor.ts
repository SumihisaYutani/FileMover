import { ExecutionSession, MovePlan } from '../types';

export class FileExecutor {
  private session: ExecutionSession;
  private plan: MovePlan;
  private isCancelled = false;
  private progressInterval?: ReturnType<typeof setTimeout>;

  constructor(sessionId: string, plan: MovePlan) {
    this.session = {
      id: sessionId,
      plan_id: plan.roots.join(','), // Simplified
      status: 'Created',
      progress: undefined,
      journal_path: this.generateJournalPath(),
      error: undefined,
    };
    this.plan = plan;
  }

  async executeMovePlan(): Promise<void> {
    this.session.status = 'Running';
    
    const totalOps = this.calculateTotalOperations();
    const totalBytes = this.plan.summary.total_bytes || 0;
    
    this.session.progress = {
      current_item: undefined,
      completed_ops: 0,
      total_ops: totalOps,
      bytes_processed: 0,
      total_bytes: totalBytes,
      current_speed: undefined,
      eta: undefined,
    };

    try {
      await this.simulateExecution();
      
      if (this.isCancelled) {
        this.session.status = 'Cancelled';
      } else {
        this.session.status = 'Completed';
        this.session.progress!.completed_ops = totalOps;
        this.session.progress!.bytes_processed = totalBytes;
        this.session.progress!.current_item = undefined;
        this.session.progress!.current_speed = undefined;
        this.session.progress!.eta = undefined;
      }
    } catch (error) {
      this.session.status = 'Failed';
      this.session.error = error instanceof Error ? error.message : String(error);
    }
  }

  private async simulateExecution(): Promise<void> {
    const totalOps = this.session.progress!.total_ops;
    // const _totalBytes = this.session.progress!.total_bytes || 0;
    let completedOps = 0;
    let processedBytes = 0;

    // Create a list of file operations to simulate
    const operations = this.generateFileOperations();
    
    for (let i = 0; i < operations.length && !this.isCancelled; i++) {
      const operation = operations[i];
      
      // Update current item
      this.session.progress!.current_item = operation.path;
      
      // Simulate operation time based on file size
      const operationTime = this.calculateOperationTime(operation.size);
      await this.delay(operationTime);
      
      if (this.isCancelled) break;
      
      // Update progress
      completedOps++;
      processedBytes += operation.size;
      
      const progress = this.session.progress!;
      progress.completed_ops = completedOps;
      progress.bytes_processed = processedBytes;
      
      // Calculate speed and ETA
      const progressRatio = completedOps / totalOps;
      if (progressRatio > 0) {
        const elapsedTime = (Date.now() - this.getStartTime()) / 1000;
        const speed = completedOps / elapsedTime;
        const remainingOps = totalOps - completedOps;
        const eta = remainingOps / speed;
        
        progress.current_speed = speed;
        progress.eta = eta;
      }
      
      // Simulate occasional failures
      if (Math.random() < 0.02) { // 2% failure rate
        console.warn(`Simulated failure for operation: ${operation.path}`);
        // In a real implementation, this would be logged to the journal
      }
    }
  }

  private generateFileOperations(): Array<{path: string, size: number}> {
    const operations: Array<{path: string, size: number}> = [];
    
    // Generate realistic file operations based on the plan
    for (const rootId of this.plan.roots) {
      const node = this.plan.nodes[rootId];
      if (!node) continue;
      
      // Simulate files within each directory
      const fileCount = Math.floor(Math.random() * 30) + 5; // 5-35 files per directory
      const totalSize = node.size_bytes || 1024 * 1024 * 50; // Default 50MB
      
      for (let i = 0; i < fileCount; i++) {
        const fileName = this.generateFileName(i, node.name_before);
        const filePath = `${node.path_before}\\${fileName}`;
        const fileSize = Math.floor(totalSize / fileCount * (0.5 + Math.random())); // Vary file sizes
        
        operations.push({
          path: filePath,
          size: fileSize,
        });
      }
    }
    
    return operations;
  }

  private generateFileName(index: number, baseName: string): string {
    const extensions = ['.jpg', '.png', '.pdf', '.txt', '.docx', '.xlsx', '.mp4', '.mp3'];
    const extension = extensions[Math.floor(Math.random() * extensions.length)];
    
    if (baseName.toLowerCase().includes('photo') || baseName.toLowerCase().includes('vacation')) {
      return `IMG_${String(index + 1).padStart(3, '0')}${extension}`;
    } else if (baseName.toLowerCase().includes('project') || baseName.toLowerCase().includes('code')) {
      const fileNames = ['index.html', 'style.css', 'script.js', 'README.md', 'package.json'];
      return fileNames[index % fileNames.length];
    } else {
      return `file_${index + 1}${extension}`;
    }
  }

  private calculateOperationTime(fileSize: number): number {
    // Simulate operation time based on file size
    // Base time: 50ms per operation
    // Additional time: 1ms per 1KB
    const baseTime = 50;
    const sizeTime = fileSize / 1024;
    const randomFactor = 0.5 + Math.random(); // Add some randomness
    
    return Math.floor((baseTime + sizeTime) * randomFactor);
  }

  private calculateTotalOperations(): number {
    let totalOps = 0;
    
    for (const rootId of this.plan.roots) {
      const node = this.plan.nodes[rootId];
      if (node && node.kind !== 'Skip' && node.kind !== 'None') {
        // Estimate operations based on directory size
        const estimatedFiles = Math.floor((node.size_bytes || 1024 * 1024 * 10) / (1024 * 1024)) + 5;
        totalOps += estimatedFiles;
      }
    }
    
    return Math.max(totalOps, 10); // Minimum 10 operations
  }

  private generateJournalPath(): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `C:\\Users\\User\\AppData\\Local\\FileMover\\journal_${timestamp}.json`;
  }

  private getStartTime(): number {
    // In a real implementation, this would be stored when execution starts
    return Date.now() - 1000; // Simulate 1 second ago
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  async cancel(): Promise<void> {
    this.isCancelled = true;
    if (this.progressInterval) {
      clearInterval(this.progressInterval);
    }
  }

  getSession(): ExecutionSession {
    return this.session;
  }
}