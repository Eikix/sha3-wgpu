import { useState, useEffect } from "react";
import "./App.css";
import BasicDemo from "./components/BasicDemo";
import PerformanceDemo from "./components/PerformanceDemo";
import AllVariantsDemo from "./components/AllVariantsDemo";

type Tab = "basic" | "performance" | "variants";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("basic");
  const [webgpuSupported, setWebgpuSupported] = useState<boolean | null>(null);

  useEffect(() => {
    // Check WebGPU support
    const checkWebGPU = async () => {
      if (!navigator.gpu) {
        setWebgpuSupported(false);
        return;
      }

      try {
        const adapter = await navigator.gpu.requestAdapter();
        setWebgpuSupported(adapter !== null);
      } catch (error) {
        console.error("WebGPU check failed:", error);
        setWebgpuSupported(false);
      }
    };

    checkWebGPU();
  }, []);

  return (
    <div className="app">
      <header className="header">
        <h1>SHA-3 WebGPU Demo</h1>
        <p>GPU-accelerated SHA-3 hashing in your browser</p>
      </header>

      {webgpuSupported !== null && (
        <div
          className={`webgpu-check ${webgpuSupported ? "supported" : "not-supported"}`}
        >
          {webgpuSupported ? (
            <>
              <strong>WebGPU is supported!</strong>
              <span>
                Your browser supports WebGPU. You can run the GPU-accelerated
                demos.
              </span>
            </>
          ) : (
            <>
              <strong>WebGPU is not supported</strong>
              <span>
                Your browser doesn't support WebGPU. Please use Chrome/Edge 113+
                or other browsers with WebGPU enabled. Visit{" "}
                <a
                  href="https://webgpu.io"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "#fca5a5" }}
                >
                  webgpu.io
                </a>{" "}
                for more information.
              </span>
            </>
          )}
        </div>
      )}

      <div className="tabs">
        <button
          className={`tab-button ${activeTab === "basic" ? "active" : ""}`}
          onClick={() => setActiveTab("basic")}
          disabled={!webgpuSupported}
        >
          Basic Usage
        </button>
        <button
          className={`tab-button ${activeTab === "performance" ? "active" : ""}`}
          onClick={() => setActiveTab("performance")}
          disabled={!webgpuSupported}
        >
          Performance
        </button>
        <button
          className={`tab-button ${activeTab === "variants" ? "active" : ""}`}
          onClick={() => setActiveTab("variants")}
          disabled={!webgpuSupported}
        >
          All Variants
        </button>
      </div>

      <div className="demo-section">
        {activeTab === "basic" && <BasicDemo />}
        {activeTab === "performance" && <PerformanceDemo />}
        {activeTab === "variants" && <AllVariantsDemo />}
      </div>
    </div>
  );
}

export default App;
