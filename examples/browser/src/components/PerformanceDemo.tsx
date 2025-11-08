import { useState } from 'react';
import { useHasher } from '../hooks/useHasher';

interface BenchmarkResult {
  batchSize: number;
  cpuTime: number;
  gpuTime: number;
  speedup: number;
  throughput: number;
}

function PerformanceDemo() {
  const [batchSizes] = useState([10, 50, 100, 500, 1000]);
  const [results, setResults] = useState<BenchmarkResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentBatch, setCurrentBatch] = useState<number | null>(null);

  const { hasher, error: hasherError } = useHasher('sha3-256');

  const benchmarkCPU = async (inputs: Uint8Array[]): Promise<number> => {
    const start = performance.now();

    // Use Web Crypto API for CPU benchmarking
    for (const input of inputs) {
      await crypto.subtle.digest('SHA-256', input); // Note: SHA-256, not SHA-3
    }

    const end = performance.now();
    return end - start;
  };

  const benchmarkGPU = async (inputs: Uint8Array[]): Promise<number> => {
    if (!hasher) throw new Error('Hasher not initialized');

    const start = performance.now();
    await hasher.hashBatch(inputs);
    const end = performance.now();

    return end - start;
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
      const cpuTime = await benchmarkCPU(inputs);

      // GPU benchmark
      const gpuTime = await benchmarkGPU(inputs);

      // Calculate metrics
      const speedup = cpuTime / gpuTime;
      const throughput = batchSize / (gpuTime / 1000);

      benchResults.push({
        batchSize,
        cpuTime,
        gpuTime,
        speedup,
        throughput
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
      <h2>Performance Comparison</h2>
      <p>
        Compare GPU-accelerated SHA-3 performance against CPU. The GPU excels at batch processing,
        showing significant speedups with larger batch sizes (100+ hashes).
      </p>
      <p style={{ fontSize: '0.9em', color: '#9ca3af' }}>
        Note: CPU benchmark uses Web Crypto API's SHA-256 (not SHA-3) for comparison purposes.
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
              <th>CPU Time</th>
              <th>GPU Time</th>
              <th>Speedup</th>
              <th>GPU Throughput</th>
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
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {results.length > 0 && (
        <div style={{ marginTop: '20px', padding: '15px', background: '#0a0a0a', borderRadius: '8px', border: '1px solid #333' }}>
          <div className="output-line info">Benchmark Complete!</div>
          <div className="output-line">
            Note: GPU performance improves significantly with larger batch sizes.
            The overhead of GPU initialization is amortized across more hashes.
          </div>
        </div>
      )}
    </div>
  );
}

export default PerformanceDemo;
