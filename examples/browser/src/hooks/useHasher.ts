import { useState, useEffect } from 'react';

type Sha3Variant = 'sha3-224' | 'sha3-256' | 'sha3-384' | 'sha3-512' | 'shake128' | 'shake256';

interface Sha3WasmHasher {
  hashSingle(input: Uint8Array): Promise<Uint8Array>;
  hashBatch(inputs: Uint8Array[]): Promise<Uint8Array[]>;
  hashBatchWithLength(inputs: Uint8Array[], outputLength: number): Promise<Uint8Array[]>;
  getVariant(): string;
  getOutputSize(): number;
}

export function useHasher(variant: Sha3Variant) {
  const [hasher, setHasher] = useState<Sha3WasmHasher | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;

    const initHasher = async () => {
      try {
        setLoading(true);
        setError(null);
        console.log('Initializing SHA-3 hasher...');

        // Import and initialize the WASM module
        console.log('Loading WASM module...');
        const module = await import('../../../../pkg/sha3_wasm.js');
        console.log('WASM module loaded, initializing...');
        await module.default();
        console.log('WASM module initialized');

        if (!mounted) return;

        // Create the hasher instance
        console.log(`Creating hasher for variant: ${variant}`);
        const hasherInstance = await new module.Sha3WasmHasher(variant);
        console.log('Hasher created successfully');

        if (!mounted) return;

        setHasher(hasherInstance);
        console.log('Hasher state updated');
      } catch (err) {
        if (!mounted) return;
        console.error('Failed to initialize hasher:', err);
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error('Error details:', errorMessage);
        setError(errorMessage);
      } finally {
        if (mounted) {
          setLoading(false);
          console.log('Hasher initialization complete, loading:', false);
        }
      }
    };

    initHasher();

    return () => {
      mounted = false;
    };
  }, [variant]);

  return { hasher, error, loading };
}
