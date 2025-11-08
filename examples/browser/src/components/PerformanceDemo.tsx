import { useState } from 'react';
import { useHasher } from '../hooks/useHasher';
import { sha3_256 } from 'js-sha3';

interface BenchmarkResult {
  batchSize: number;
  cpuTime: number;
  gpuTime: number;
  speedup: number;
  throughput: number;
  verified: boolean;
  sampleHash?: string;
}

function PerformanceDemo() {
  const [batchSizes] = useState([10, 50, 100, 500, 1000]);
  const [results, setResults] = useState<BenchmarkResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentBatch, setCurrentBatch] = useState<number | null>(null);

  const { hasher, error: hasherError } = useHasher('sha3-256');

  const benchmarkCPU = async (inputs: Uint8Array[]): Promise<{ time: number; hashes: string[] }> => {
    const start = performance.now();

    // Use js-sha3 for CPU SHA-3 benchmarking
    const hashes: string[] = [];
    for (const input of inputs) {
      const hash = sha3_256(input);
      hashes.push(hash);
    }

    const end = performance.now();
    return { time: end - start, hashes };
  };

  const benchmarkGPU = async (inputs: Uint8Array[]): Promise<{ time: number; hashes: Uint8Array[] }> => {
    if (!hasher) throw new Error('Hasher not initialized');

    const start = performance.now();
    const hashes = await hasher.hashBatch(inputs);
    const end = performance.now();

    return { time: end - start, hashes };
  };

  const runBenchmark = async () => {
    if (!hasher) return;

    setLoading(true);
    setResults([]);

    const benchResults: BenchmarkResult[] = [];

    for (const batchSize of batchSizes) {
      setCurrentBatch(batchSize);

      // Generate test data (64 bytes per input)
      const inputs: Uint8Array[] = [];
      for (let i = 0; i < batchSize; i++) {
        const data = new Uint8Array(64);
        const text = `test input number ${i}`;
        const encoded = new TextEncoder().encode(text);
        data.set(encoded);
        inputs.push(data);
      }

      // Warmup
      await benchmarkGPU(inputs);

      // CPU benchmark
      const cpuResult = await benchmarkCPU(inputs);

      // GPU benchmark
      const gpuResult = await benchmarkGPU(inputs);

      // Verify results match between CPU and GPU
      let verified = true;
      let sampleHash = '';

      for (let i = 0; i < inputs.length; i++) {
        const gpuHashHex = Array.from(gpuResult.hashes[i])
          .map(b => b.toString(16).padStart(2, '0'))
          .join('');
        const cpuHash = cpuResult.hashes[i];

        if (i === 0) {
          sampleHash = gpuHashHex; // Store first hash as sample
        }

        if (gpuHashHex !== cpuHash) {
          verified = false;
          console.error(`Hash mismatch at index ${i}:`, {
            gpu: gpuHashHex,
            cpu: cpuHash,
            input: new TextDecoder().decode(inputs[i])
          });
          break;
        }
      }

      // Calculate metrics
      const speedup = cpuResult.time / gpuResult.time;
      const throughput = batchSize / (gpuResult.time / 1000);

      benchResults.push({
        batchSize,
        cpuTime: cpuResult.time,
        gpuTime: gpuResult.time,
        speedup,
        throughput,
        verified,
        sampleHash
      });

      setResults([...benchResults]);
    }

    setCurrentBatch(null);
    setLoading(false);
  };

  if (hasherError) {
    return (
      <div>
        <h2>Performance Comparison</h2>
        <div className="output">
          <div className="output-line error">Error initializing hasher: {hasherError}</div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <h2>GPU vs CPU SHA-3 Comparison</h2>
      <p>
        Compare GPU-accelerated SHA-3 performance against CPU SHA-3 (using js-sha3 library).
        This benchmark tests both performance and correctness by verifying that GPU and CPU
        implementations produce identical hash outputs.
      </p>
      <p style={{ fontSize: '0.9em', color: '#9ca3af' }}>
        The GPU excels at batch processing, showing significant speedups with larger batch sizes (100+ hashes).
      </p>

      <div className="controls">
        <button onClick={runBenchmark} disabled={loading || !hasher}>
          Run Benchmark
          {loading && <span className="loading"></span>}
        </button>
        {loading && currentBatch && (
          <span style={{ color: '#9ca3af' }}>
            Testing batch size: {currentBatch}
          </span>
        )}
      </div>

      {results.length > 0 && (
        <table className="performance-table">
          <thead>
            <tr>
              <th>Batch Size</th>
              <th>CPU Time (SHA-3)</th>
              <th>GPU Time (SHA-3)</th>
              <th>Speedup</th>
              <th>GPU Throughput</th>
              <th>Results Match</th>
            </tr>
          </thead>
          <tbody>
            {results.map((result) => (
              <tr key={result.batchSize}>
                <td>{result.batchSize}</td>
                <td>{result.cpuTime.toFixed(2)} ms</td>
                <td>{result.gpuTime.toFixed(2)} ms</td>
                <td className={result.speedup >= 1 ? 'speedup-good' : 'speedup-bad'}>
                  {result.speedup.toFixed(2)}x
                </td>
                <td>{result.throughput.toFixed(0)} hashes/sec</td>
                <td>
                  <span style={{
                    color: result.verified ? '#22c55e' : '#ef4444',
                    fontWeight: 'bold'
                  }}>
                    {result.verified ? '✓ PASS' : '✗ FAIL'}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {results.length > 0 && (
        <div style={{ marginTop: '20px', padding: '15px', background: '#0a0a0a', borderRadius: '8px', border: '1px solid #333' }}>
          <div className="output-line info">Benchmark Complete!</div>

          {/* Verification Summary */}
          <div style={{ marginTop: '15px', padding: '10px', background: '#1a1a1a', borderRadius: '4px' }}>
            <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>Verification Summary:</div>
            {results.every(r => r.verified) ? (
              <div style={{ color: '#22c55e' }}>
                ✓ All GPU hashes match CPU SHA-3 hashes - Implementation is correct!
              </div>
            ) : (
              <div style={{ color: '#ef4444' }}>
                ✗ Some hashes don't match - Check console for details
              </div>
            )}
          </div>

          {/* Performance Summary */}
          <div style={{ marginTop: '10px', padding: '10px', background: '#1a1a1a', borderRadius: '4px' }}>
            <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>Performance Notes:</div>
            <div className="output-line">
              • GPU performance improves significantly with larger batch sizes
            </div>
            <div className="output-line">
              • GPU initialization overhead is amortized across more hashes
            </div>
            <div className="output-line">
              • CPU time is for single-threaded JavaScript SHA-3 (js-sha3 library)
            </div>
          </div>

          {/* Sample Hash */}
          {results[0]?.sampleHash && (
            <div style={{ marginTop: '10px', padding: '10px', background: '#1a1a1a', borderRadius: '4px' }}>
              <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>Sample Hash (first input):</div>
              <code style={{ fontSize: '0.85em', wordBreak: 'break-all', color: '#60a5fa' }}>
                {results[0].sampleHash}
              </code>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default PerformanceDemo;
