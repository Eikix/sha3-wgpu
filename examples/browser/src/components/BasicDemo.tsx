import { useState } from 'react';
import { useHasher } from '../hooks/useHasher';

function BasicDemo() {
  const [inputText, setInputText] = useState('Hello, GPU-accelerated SHA-3!');
  const [batchInputs, setBatchInputs] = useState('message 1\nmessage 2\nmessage 3\nmessage 4\nmessage 5');
  const [output, setOutput] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const { hasher, error: hasherError } = useHasher('sha3-256');

  const handleSingleHash = async () => {
    if (!hasher) return;

    setLoading(true);
    setOutput([]);

    try {
      setOutput(prev => [...prev, '=== Single Hash ===']);
      setOutput(prev => [...prev, `Input: "${inputText}"`]);

      const input = new TextEncoder().encode(inputText);
      const hash = await hasher.hashSingle(input);
      const hashHex = Array.from(hash)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      setOutput(prev => [...prev, `Hash: ${hashHex}`, '']);
    } catch (error) {
      setOutput(prev => [...prev, `Error: ${error}`, '']);
    } finally {
      setLoading(false);
    }
  };

  const handleBatchHash = async () => {
    if (!hasher) return;

    setLoading(true);
    setOutput([]);

    try {
      const messages = batchInputs.split('\n').filter(m => m.trim());
      setOutput(prev => [...prev, '=== Batch Hashing ===']);
      setOutput(prev => [...prev, `Hashing ${messages.length} messages in one GPU batch...`, '']);

      const inputs = messages.map(msg => new TextEncoder().encode(msg));
      const hashes = await hasher.hashBatch(inputs);

      hashes.forEach((hash, i) => {
        const hashHex = Array.from(hash)
          .map(b => b.toString(16).padStart(2, '0'))
          .join('');
        setOutput(prev => [...prev, `${i + 1}. "${messages[i]}" => ${hashHex}`]);
      });

      setOutput(prev => [...prev, '', `Successfully hashed ${hashes.length} messages on GPU!`]);
    } catch (error) {
      setOutput(prev => [...prev, `Error: ${error}`, '']);
    } finally {
      setLoading(false);
    }
  };

  if (hasherError) {
    return (
      <div>
        <h2>Basic Usage</h2>
        <div className="output">
          <div className="output-line error">Error initializing hasher: {hasherError}</div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <h2>Basic Usage</h2>
      <p>Demonstrate single hash and batch hashing using GPU-accelerated SHA-3.</p>

      <div className="controls">
        <div className="control-group">
          <label>Single Hash Input:</label>
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            style={{ width: '400px' }}
            disabled={loading || !hasher}
          />
        </div>
        <button onClick={handleSingleHash} disabled={loading || !hasher || !inputText}>
          Hash Single
          {loading && <span className="loading"></span>}
        </button>
      </div>

      <div className="controls">
        <div className="control-group">
          <label>Batch Inputs (one per line):</label>
          <textarea
            value={batchInputs}
            onChange={(e) => setBatchInputs(e.target.value)}
            style={{
              width: '400px',
              height: '100px',
              background: '#0a0a0a',
              border: '1px solid #333',
              color: '#e0e0e0',
              padding: '8px 12px',
              borderRadius: '6px',
              fontFamily: 'monospace',
              fontSize: '14px'
            }}
            disabled={loading || !hasher}
          />
        </div>
        <button onClick={handleBatchHash} disabled={loading || !hasher || !batchInputs.trim()}>
          Hash Batch
          {loading && <span className="loading"></span>}
        </button>
      </div>

      {output.length > 0 && (
        <div className="output">
          {output.map((line, i) => (
            <div
              key={i}
              className={`output-line ${
                line.startsWith('===') ? 'info' :
                line.includes('=>') ? 'hash' :
                line.startsWith('Error') ? 'error' :
                line.startsWith('Successfully') ? 'success' :
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

export default BasicDemo;
