import { useMemo, useState } from "react";
import { sha3_256 } from "js-sha3";
import { useHasher } from "../hooks/useHasher";

type Scenario = {
  id: string;
  label: string;
  leaves: number;
  runActual: boolean;
  description: string;
};

type ScenarioResult = {
  scenarioId: string;
  label: string;
  leaves: number;
  levels: number;
  totalPairs: number;
  totalBytes: number;
  cpuTimeMs: number;
  gpuTimeMs: number;
  speedup: number;
  gpuThroughputMiBs: number;
  estimated: boolean;
  note?: string;
};

const SCENARIOS: Scenario[] = [
  {
    id: "demo-4k",
    label: "4,096 leaves (2^12)",
    leaves: 4096,
    runActual: true,
    description:
      "Fits easily on most GPUs – good for verifying correctness and pipeline overhead.",
  },
  {
    id: "demo-65k",
    label: "65,536 leaves (2^16)",
    leaves: 65_536,
    runActual: true,
    description:
      "Stresses batching – roughly 1 MB of parent hashing work per level.",
  },
  {
    id: "projection-1m",
    label: "1,048,576 leaves (2^20)",
    leaves: 1_048_576,
    runActual: true,
    description:
      "Matches a 1M-leaf Merkle tree. Uses measured throughput to project timings.",
  },
];

// Derived from Criterion results (sha3_large_batch, 10k inputs @ 64 bytes)
const CPU_BYTES_PER_MS = 389_835; // ~372 MiB/s
const GPU_BYTES_PER_MS = 209_706; // ~200 MiB/s
const GPU_DISPATCH_OVERHEAD_MS = 2.9;
const INTERNAL_NODE_SIZE = 64; // Each parent hashes two 32-byte children
const LEAF_SIZE = 32;

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

function generateLeaves(count: number): Uint8Array[] {
  return Array.from({ length: count }, (_, i) => {
    const leaf = new Uint8Array(LEAF_SIZE);
    for (let j = 0; j < LEAF_SIZE; j += 1) {
      leaf[j] = (i + j) & 0xff;
    }
    return leaf;
  });
}

function buildMerkleCPU(leaves: Uint8Array[]) {
  let level: Uint8Array[] = leaves.map((leaf) => leaf.slice());
  let totalPairs = 0;
  let totalBytes = 0;
  let levels = 0;

  const start = performance.now();
  while (level.length > 1) {
    const nextLevel: Uint8Array[] = [];
    for (let i = 0; i < level.length; i += 2) {
      const left = level[i];
      const right = level[i + 1] ?? level[i];
      const combined = new Uint8Array(left.length + right.length);
      combined.set(left);
      combined.set(right, left.length);

      const hashHex = sha3_256(combined);
      nextLevel.push(hexToBytes(hashHex));

      totalPairs += 1;
      totalBytes += combined.length;
    }
    levels += 1;
    level = nextLevel;
  }
  const end = performance.now();

  return {
    timeMs: end - start,
    totalPairs,
    totalBytes,
    levels,
    root: level[0] ?? new Uint8Array(LEAF_SIZE),
  };
}

async function buildMerkleGPU(
  hasher: {
    hashBatch(inputs: Uint8Array[]): Promise<Uint8Array[]>;
  },
  leaves: Uint8Array[],
) {
  let level: Uint8Array[] = leaves.map((leaf) => leaf.slice());
  let totalPairs = 0;
  let totalBytes = 0;
  let levels = 0;

  const start = performance.now();
  while (level.length > 1) {
    const nextInputCount = Math.ceil(level.length / 2);
    const inputs: Uint8Array[] = new Array(nextInputCount);

    for (let i = 0; i < level.length; i += 2) {
      const left = level[i];
      const right = level[i + 1] ?? level[i];
      const combined = new Uint8Array(left.length + right.length);
      combined.set(left);
      combined.set(right, left.length);

      inputs[Math.floor(i / 2)] = combined;
      totalPairs += 1;
      totalBytes += combined.length;
    }

    const hashes = await hasher.hashBatch(inputs);
    level = hashes;
    levels += 1;
  }
  const end = performance.now();

  return {
    timeMs: end - start,
    totalPairs,
    totalBytes,
    levels,
    root: level[0] ?? new Uint8Array(LEAF_SIZE),
  };
}

function formatMiB(bytes: number, digits = 2) {
  return (bytes / (1024 * 1024)).toFixed(digits);
}

function formatNumber(value: number) {
  return value.toLocaleString("en-US");
}

function formatMs(value: number) {
  return `${value.toFixed(2)} ms`;
}

function MerkleTreeDemo() {
  const { hasher, error: hasherError } = useHasher("sha3-256");
  const [loadingScenario, setLoadingScenario] = useState<string | null>(null);
  const [results, setResults] = useState<ScenarioResult[]>([]);

  const scenarioMap = useMemo(() => {
    const map = new Map<string, ScenarioResult>();
    for (const res of results) {
      map.set(res.scenarioId, res);
    }
    return map;
  }, [results]);

  const handleRunScenario = async (scenario: Scenario) => {
    if (!scenario.runActual && scenarioMap.has(scenario.id)) {
      return;
    }

    if (!hasher && scenario.runActual) {
      return;
    }

    setLoadingScenario(scenario.id);

    try {
      let result: ScenarioResult;

      if (scenario.runActual && hasher) {
        const leaves = generateLeaves(scenario.leaves);

        const cpuMetrics = buildMerkleCPU(leaves);
        const gpuMetrics = await buildMerkleGPU(hasher, leaves);

        const totalBytes = gpuMetrics.totalBytes;
        const gpuThroughput =
          totalBytes > 0
            ? totalBytes / (1024 * 1024) / (gpuMetrics.timeMs / 1000)
            : 0;

        result = {
          scenarioId: scenario.id,
          label: scenario.label,
          leaves: scenario.leaves,
          levels: gpuMetrics.levels,
          totalPairs: gpuMetrics.totalPairs,
          totalBytes: gpuMetrics.totalBytes,
          cpuTimeMs: cpuMetrics.timeMs,
          gpuTimeMs: gpuMetrics.timeMs,
          speedup: cpuMetrics.timeMs / gpuMetrics.timeMs,
          gpuThroughputMiBs: gpuThroughput,
          estimated: false,
          note: "Measured directly in browser",
        };
      } else {
        const leaves = scenario.leaves;
        const levels = Math.ceil(Math.log2(leaves)) + 1;
        const totalPairs = leaves - 1;
        const totalBytes = totalPairs * INTERNAL_NODE_SIZE;
        const cpuTimeMs = totalBytes / CPU_BYTES_PER_MS;
        const gpuTimeMs =
          GPU_DISPATCH_OVERHEAD_MS + totalBytes / GPU_BYTES_PER_MS;
        const gpuThroughput =
          totalBytes > 0 ? totalBytes / (1024 * 1024) / (gpuTimeMs / 1000) : 0;

        result = {
          scenarioId: scenario.id,
          label: scenario.label,
          leaves,
          levels,
          totalPairs,
          totalBytes,
          cpuTimeMs,
          gpuTimeMs,
          speedup: cpuTimeMs / gpuTimeMs,
          gpuThroughputMiBs: gpuThroughput,
          estimated: true,
          note: "Estimated from Criterion throughput (no browser run)",
        };
      }

      setResults((prev) => {
        const filtered = prev.filter((res) => res.scenarioId !== scenario.id);
        return [...filtered, result].sort((a, b) => a.leaves - b.leaves);
      });
    } finally {
      setLoadingScenario(null);
    }
  };

  return (
    <div>
      <h2>Merkle Tree Accelerator</h2>
      <p>
        Build layered SHA3-256 Merkle trees using the GPU batch hasher. Each
        internal node hashes two 32-byte children into a new 32-byte parent (64
        bytes per hash call). Compare measured browser timings with projections
        for much larger trees.
      </p>

      {hasherError && (
        <div className="output-line error">
          Failed to initialize GPU hasher: {hasherError}
        </div>
      )}

      <div
        className="controls"
        style={{ flexDirection: "column", gap: "12px" }}
      >
        {SCENARIOS.map((scenario) => {
          const existing = scenarioMap.get(scenario.id);
          const buttonDisabled =
            (scenario.runActual &&
              (!hasher || loadingScenario === scenario.id)) ||
            (!scenario.runActual && loadingScenario === scenario.id);
          return (
            <div
              key={scenario.id}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "12px",
                background: "#0a0a0a",
                border: "1px solid #1f2937",
                borderRadius: "8px",
                padding: "12px 16px",
              }}
            >
              <div>
                <div style={{ fontWeight: "bold" }}>{scenario.label}</div>
                <div style={{ fontSize: "0.85em", color: "#9ca3af" }}>
                  {scenario.description}
                </div>
                <div style={{ fontSize: "0.85em", color: "#9ca3af" }}>
                  Leaves: {formatNumber(scenario.leaves)} &nbsp;|&nbsp; Data in:{" "}
                  {formatMiB(scenario.leaves * LEAF_SIZE)} MiB
                </div>
              </div>
              <div
                style={{ display: "flex", alignItems: "center", gap: "8px" }}
              >
                {existing && (
                  <span
                    style={{
                      fontSize: "0.8em",
                      color: existing.estimated ? "#f59e0b" : "#22c55e",
                    }}
                  >
                    {existing.estimated ? "Estimated" : "Measured"}
                  </span>
                )}
                <button
                  onClick={() => handleRunScenario(scenario)}
                  disabled={buttonDisabled}
                >
                  {loadingScenario === scenario.id
                    ? "Running…"
                    : existing
                      ? "Re-run"
                      : "Run"}
                </button>
              </div>
            </div>
          );
        })}
      </div>

      {results.length > 0 && (
        <table className="performance-table" style={{ marginTop: "20px" }}>
          <thead>
            <tr>
              <th>Scenario</th>
              <th>Leaves</th>
              <th>Levels</th>
              <th>Total Pairs</th>
              <th>Total Bytes</th>
              <th>CPU Time</th>
              <th>GPU Time</th>
              <th>CPU/GPU</th>
              <th>GPU Throughput</th>
              <th>Notes</th>
            </tr>
          </thead>
          <tbody>
            {results.map((result) => (
              <tr key={result.scenarioId}>
                <td>{result.label}</td>
                <td>{formatNumber(result.leaves)}</td>
                <td>{result.levels}</td>
                <td>{formatNumber(result.totalPairs)}</td>
                <td>{formatMiB(result.totalBytes)} MiB</td>
                <td>{formatMs(result.cpuTimeMs)}</td>
                <td>{formatMs(result.gpuTimeMs)}</td>
                <td
                  className={
                    result.speedup >= 1 ? "speedup-good" : "speedup-bad"
                  }
                >
                  {result.speedup.toFixed(2)}x
                </td>
                <td>{result.gpuThroughputMiBs.toFixed(1)} MiB/s</td>
                <td style={{ color: result.estimated ? "#f59e0b" : "#9ca3af" }}>
                  {result.note}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

export default MerkleTreeDemo;
