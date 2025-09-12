import { ScanSession, PlanSession, ExecutionSession, Rule, ScanOptions } from '../types';
import { FileScanner } from './fileScanner';
import { PlanGenerator } from './planGenerator';
import { FileExecutor } from './fileExecutor';

class SessionManager {
  private scanSessions = new Map<string, ScanSession>();
  private planSessions = new Map<string, PlanSession>();
  private executionSessions = new Map<string, ExecutionSession>();
  private scanners = new Map<string, FileScanner>();
  private planGenerators = new Map<string, PlanGenerator>();
  private executors = new Map<string, FileExecutor>();

  // Scan session methods
  createScanSession(roots: string[], _options: ScanOptions): string {
    const sessionId = `scan-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    
    const session: ScanSession = {
      id: sessionId,
      roots,
      status: 'Created',
      progress: undefined,
      results: undefined,
      error: undefined,
    };
    
    this.scanSessions.set(sessionId, session);
    return sessionId;
  }

  async startScan(sessionId: string, rules: Rule[], options: ScanOptions): Promise<void> {
    const session = this.scanSessions.get(sessionId);
    if (!session) {
      throw new Error('Scan session not found');
    }

    const scanner = new FileScanner(sessionId, session.roots, rules, options);
    this.scanners.set(sessionId, scanner);
    
    await scanner.startScan();
    
    // Update session with results
    const updatedSession = scanner.getSession();
    this.scanSessions.set(sessionId, updatedSession);
  }

  getScanSession(sessionId: string): ScanSession | undefined {
    return this.scanSessions.get(sessionId);
  }

  // Plan session methods
  createPlanSession(scanId: string, _selectedPaths: string[]): string {
    const sessionId = `plan-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    
    const session: PlanSession = {
      id: sessionId,
      scan_id: scanId,
      status: 'Created',
      plan: undefined,
      error: undefined,
    };
    
    this.planSessions.set(sessionId, session);
    return sessionId;
  }

  async startPlanning(sessionId: string): Promise<void> {
    const session = this.planSessions.get(sessionId);
    if (!session) {
      throw new Error('Plan session not found');
    }

    const scanSession = session.scan_id ? this.scanSessions.get(session.scan_id) : undefined;
    if (!scanSession?.results) {
      throw new Error('Scan session or results not found');
    }

    const planGenerator = new PlanGenerator(sessionId, scanSession.results);
    this.planGenerators.set(sessionId, planGenerator);
    
    session.status = 'Running';
    this.planSessions.set(sessionId, session);
    
    await planGenerator.generatePlan();
    
    const updatedSession = planGenerator.getSession();
    this.planSessions.set(sessionId, updatedSession);
  }

  getPlanSession(sessionId: string): PlanSession | undefined {
    return this.planSessions.get(sessionId);
  }

  // Execution session methods
  createExecutionSession(planId: string): string {
    const sessionId = `exec-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    
    const session: ExecutionSession = {
      id: sessionId,
      plan_id: planId,
      status: 'Created',
      progress: undefined,
      journal_path: undefined,
      error: undefined,
    };
    
    this.executionSessions.set(sessionId, session);
    return sessionId;
  }

  async startExecution(sessionId: string): Promise<void> {
    const session = this.executionSessions.get(sessionId);
    if (!session) {
      throw new Error('Execution session not found');
    }

    const planSession = session.plan_id ? this.planSessions.get(session.plan_id) : undefined;
    if (!planSession?.plan) {
      throw new Error('Plan session or plan not found');
    }

    const executor = new FileExecutor(sessionId, planSession.plan);
    this.executors.set(sessionId, executor);
    
    await executor.executeMovePlan();
    
    const updatedSession = executor.getSession();
    this.executionSessions.set(sessionId, updatedSession);
  }

  getExecutionSession(sessionId: string): ExecutionSession | undefined {
    return this.executionSessions.get(sessionId);
  }

  async cancelExecution(sessionId: string): Promise<void> {
    const executor = this.executors.get(sessionId);
    if (executor) {
      await executor.cancel();
    }
    
    const session = this.executionSessions.get(sessionId);
    if (session) {
      session.status = 'Cancelled';
      this.executionSessions.set(sessionId, session);
    }
  }

  // Utility methods
  getAllScanSessions(): ScanSession[] {
    return Array.from(this.scanSessions.values());
  }

  getAllPlanSessions(): PlanSession[] {
    return Array.from(this.planSessions.values());
  }

  getAllExecutionSessions(): ExecutionSession[] {
    return Array.from(this.executionSessions.values());
  }

  // Cleanup old sessions
  cleanup(olderThanMinutes: number = 60): void {
    const cutoff = Date.now() - (olderThanMinutes * 60 * 1000);
    
    // Clean up scan sessions
    for (const [id, _session] of this.scanSessions.entries()) {
      const sessionTime = parseInt(id.split('-')[1]);
      if (sessionTime < cutoff) {
        this.scanSessions.delete(id);
        this.scanners.delete(id);
      }
    }
    
    // Clean up plan sessions
    for (const [id, _session] of this.planSessions.entries()) {
      const sessionTime = parseInt(id.split('-')[1]);
      if (sessionTime < cutoff) {
        this.planSessions.delete(id);
        this.planGenerators.delete(id);
      }
    }
    
    // Clean up execution sessions
    for (const [id, _session] of this.executionSessions.entries()) {
      const sessionTime = parseInt(id.split('-')[1]);
      if (sessionTime < cutoff) {
        this.executionSessions.delete(id);
        this.executors.delete(id);
      }
    }
  }
}

// Export singleton instance
export const sessionManager = new SessionManager();

// Export class for testing
export { SessionManager };