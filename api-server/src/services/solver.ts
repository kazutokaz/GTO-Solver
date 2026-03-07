import { spawn } from 'child_process';
import { config } from '../config';

export interface SolveResult {
  job_id: string;
  status: string;
  exploitability: number;
  iterations: number;
  elapsed_seconds: number;
  solution: any;
}

/**
 * Run the CFR engine binary with the given input JSON.
 * Pipes input via stdin and reads JSON from stdout.
 */
export function runCfrEngine(input: any): Promise<SolveResult> {
  return new Promise((resolve, reject) => {
    const proc = spawn(config.cfrEngine.path, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (data: Buffer) => {
      stdout += data.toString();
    });

    proc.stderr.on('data', (data: Buffer) => {
      stderr += data.toString();
    });

    proc.on('close', (code: number | null) => {
      if (stderr) {
        console.log('[CFR Engine stderr]', stderr.trim());
      }

      if (code !== 0) {
        reject(new Error(`CFR engine exited with code ${code}: ${stderr}`));
        return;
      }

      try {
        const result = JSON.parse(stdout) as SolveResult;
        resolve(result);
      } catch (err) {
        reject(new Error(`Failed to parse CFR output: ${stdout.slice(0, 500)}`));
      }
    });

    proc.on('error', (err: Error) => {
      reject(new Error(`Failed to spawn CFR engine: ${err.message}`));
    });

    // Write input JSON to stdin
    proc.stdin.write(JSON.stringify(input));
    proc.stdin.end();
  });
}
