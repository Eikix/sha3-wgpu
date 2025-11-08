import { useState } from 'react';
import { useHasher } from '../hooks/useHasher';

type Sha3Variant = 'sha3-224' | 'sha3-256' | 'sha3-384' | 'sha3-512' | 'shake128' | 'shake256';

const VARIANTS: { value: Sha3Variant; label: string; outputSize: number }[] = [
  { value: 'sha3-224', label: 'SHA3-224', outputSize: 28 },
  { value: 'sha3-256', label: 'SHA3-256', outputSize: 32 },
  { value: 'sha3-384', label: 'SHA3-384', outputSize: 48 },
  { value: 'sha3-512', label: 'SHA3-512', outputSize: 64 },
  { value: 'shake128', label: 'SHAKE128', outputSize: 16 },
  { value: 'shake256', label: 'SHAKE256', outputSize: 32 },
];

function AllVariantsDemo() {
  const [selectedVariant, setSelectedVariant] = useState<Sha3Variant>('sha3-256');
  const [inputText, setInputText] = useState('Hello, World!');
  const [output, setOutput] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const { hasher, error: hasherError } = useHasher(selectedVariant);

  const handleHashAllVariants = async () => {
    setLoading(true);
    setOutput([]);

    try {
      setOutput(prev => [...prev, '=== Testing All SHA-3 Variants ===']);
      setOutput(prev => [...prev, `Input: "${inputText}"`, '']);

      for (const variant of VARIANTS) {
        setOutput(prev => [...prev, `Testing ${variant.label}...`]);

        // Create a hasher for this variant
        const { Sha3WasmHasher } = await import('../../../../pkg/sha3_wasm.js');
        const variantHasher = await new Sha3WasmHasher(variant.value);

        const input = new TextEncoder().encode(inputText);
        const hash = await variantHasher.hashSingle(input);
        const hashHex = Array.from(hash)
          .map(b => b.toString(16).padStart(2, '0'))
          .join('');

        setOutput(prev => [...prev, `  ${variant.label} (${variant.outputSize} bytes): ${hashHex}`, '']);
      }

      setOutput(prev => [...prev, 'All variants tested successfully!']);
    } catch (error) {
      setOutput(prev => [...prev, `Error: ${error}`, '']);
    } finally {
      setLoading(false);
    }
  };

  const handleHashSelected = async () => {
    if (!hasher) return;

    setLoading(true);
    setOutput([]);

    try {
      const variant = VARIANTS.find(v => v.value === selectedVariant);
      setOutput(prev => [...prev, `=== ${variant?.label} ===`]);
      setOutput(prev => [...prev, `Input: "${inputText}"`]);

      const input = new TextEncoder().encode(inputText);
      const hash = await hasher.hashSingle(input);
      const hashHex = Array.from(hash)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      setOutput(prev => [...prev, `Hash: ${hashHex}`]);
      setOutput(prev => [...prev, `Output size: ${hash.length} bytes`, '']);
    } catch (error) {
      setOutput(prev => [...prev, `Error: ${error}`, '']);
    } finally {
      setLoading(false);
    }
  };

  if (hasherError) {
    return (
      <div>
        <h2>All SHA-3 Variants</h2>
        <div className="output">
          <div className="output-line error">Error initializing hasher: {hasherError}</div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <h2>All SHA-3 Variants</h2>
      <p>
        Test different SHA-3 variants including SHA3-224, SHA3-256, SHA3-384, SHA3-512,
        and the extendable-output functions SHAKE128 and SHAKE256.
      </p>

      <div className="controls">
        <div className="control-group">
          <label>Input Text:</label>
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            style={{ width: '300px' }}
            disabled={loading}
          />
        </div>
        <div className="control-group">
          <label>Variant:</label>
          <select
            value={selectedVariant}
            onChange={(e) => setSelectedVariant(e.target.value as Sha3Variant)}
            disabled={loading}
          >
            {VARIANTS.map(variant => (
              <option key={variant.value} value={variant.value}>
                {variant.label} ({variant.outputSize} bytes)
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="controls">
        <button onClick={handleHashSelected} disabled={loading || !hasher || !inputText}>
          Hash with Selected Variant
          {loading && <span className="loading"></span>}
        </button>
        <button onClick={handleHashAllVariants} disabled={loading || !inputText}>
          Test All Variants
        </button>
      </div>

      {output.length > 0 && (
        <div className="output">
          {output.map((line, i) => (
            <div
              key={i}
              className={`output-line ${
                line.startsWith('===') ? 'info' :
                line.includes(': ') && line.length > 50 ? 'hash' :
                line.startsWith('Error') ? 'error' :
                line.startsWith('All variants') || line.startsWith('Successfully') ? 'success' :
                ''
              }`}
            >
              {line}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default AllVariantsDemo;
