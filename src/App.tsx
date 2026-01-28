import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { 
  Shield, 
  Wifi, 
  WifiOff, 
  Lock,
  Loader2,
  Send,
  CheckCircle2,
  AlertTriangle,
  Smartphone,
  Zap,
  Users,
  ArrowRight
} from "lucide-react";

type NetworkStatus = "offline" | "online" | "checking";
type AppView = "onboarding" | "chat" | "settings";

interface FederationStatus {
  opted_in: boolean;
  embeddings_contributed: number;
  collective_users: number;
}

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  sources?: string[];
}

interface VaultStats {
  total_messages: number;
  sources: string[];
  last_indexed: string | null;
}

interface PersonInfo {
  name: string;
  mentions: number;
  curvature: number;
}

interface RelationshipInfo {
  person1: string;
  person2: string;
  strength: number;
}

interface LatticeSnapshot {
  key_people: PersonInfo[];
  relationships: RelationshipInfo[];
  total_atoms: number;
  total_edges: number;
}

function App() {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>("checking");
  const [currentView, setCurrentView] = useState<AppView>("onboarding");
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isProcessing, setIsProcessing] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [vaultStats, setVaultStats] = useState<VaultStats>({
    total_messages: 0,
    sources: [],
    last_indexed: null,
  });
  const [federationStatus, setFederationStatus] = useState<FederationStatus>({
    opted_in: false,
    embeddings_contributed: 0,
    collective_users: 0,
  });
  const [whatsappDetected, setWhatsappDetected] = useState<boolean | null>(null);
  const [statusMessage, setStatusMessage] = useState<string>("");
  const [suggestedQueries, setSuggestedQueries] = useState<string[]>([]);
  const [lattice, setLattice] = useState<LatticeSnapshot | null>(null);

  useEffect(() => {
    // Check immediately, then again after 1s (for WiFi disable to take effect), then every 3s
    checkNetworkStatus();
    const initialCheck = setTimeout(checkNetworkStatus, 1000);
    loadVaultStats();
    loadFederationStatus();
    checkWhatsAppInstalled();
    const interval = setInterval(checkNetworkStatus, 3000);
    
    const unlisten = listen<string>("status", (event) => {
      setStatusMessage(event.payload);
    });
    
    return () => {
      clearTimeout(initialCheck);
      clearInterval(interval);
      unlisten.then(fn => fn());
    };
  }, []);

  useEffect(() => {
    if (vaultStats.total_messages > 0) {
      setCurrentView("chat");
      loadSuggestedQueries();
      loadLatticeSnapshot();
    }
  }, [vaultStats.total_messages]);

  async function loadSuggestedQueries() {
    try {
      const queries = await invoke<string[]>("get_suggested_queries");
      setSuggestedQueries(queries);
    } catch (e) {
      console.error("Failed to load suggestions:", e);
    }
  }

  async function loadLatticeSnapshot() {
    try {
      const snapshot = await invoke<LatticeSnapshot>("get_lattice_snapshot");
      setLattice(snapshot);
    } catch (e) {
      console.error("Failed to load lattice:", e);
    }
  }

  async function checkNetworkStatus() {
    try {
      const isOffline = await invoke<boolean>("check_offline_status");
      setNetworkStatus(isOffline ? "offline" : "online");
    } catch {
      setNetworkStatus("checking");
    }
  }

  async function loadVaultStats() {
    try {
      const stats = await invoke<VaultStats>("get_vault_stats");
      setVaultStats(stats);
    } catch (e) {
      console.error("Failed to load vault stats:", e);
    }
  }

  async function loadFederationStatus() {
    try {
      const status = await invoke<FederationStatus>("get_federation_status");
      setFederationStatus(status);
    } catch (e) {
      console.error("Failed to load federation status:", e);
    }
  }

  async function checkWhatsAppInstalled() {
    try {
      const status = await invoke<{ available: boolean; message_count: number }>("check_whatsapp_available");
      setWhatsappDetected(status.available);
    } catch (e) {
      console.error("WhatsApp check failed:", e);
      setWhatsappDetected(false);
    }
  }

  async function handleImportWhatsApp() {
    setIsImporting(true);
    try {
      await invoke("import_messages", { source: "whatsapp", filePath: null });
      await loadVaultStats();
      setCurrentView("chat");
    } catch (e) {
      console.error("Import failed:", e);
      alert(`Import failed: ${e}`);
    } finally {
      setIsImporting(false);
    }
  }

  async function handleOptIn() {
    try {
      const status = await invoke<FederationStatus>("opt_in_federation");
      setFederationStatus(status);
    } catch (e) {
      console.error("Failed to opt in:", e);
    }
  }

  async function handleOptOut() {
    try {
      const status = await invoke<FederationStatus>("opt_out_federation");
      setFederationStatus(status);
    } catch (e) {
      console.error("Failed to opt out:", e);
    }
  }

  async function handleSendMessage() {
    if (!inputValue.trim() || isProcessing) return;
    if (networkStatus === "online") return;

    const userMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      content: inputValue,
    };

    setMessages((prev: Message[]) => [...prev, userMessage]);
    setInputValue("");
    setIsProcessing(true);

    try {
      const response = await invoke<{ answer: string; sources: string[] }>(
        "query_vault",
        { prompt: inputValue }
      );

      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: response.answer,
        sources: response.sources,
      };

      setMessages((prev: Message[]) => [...prev, assistantMessage]);
    } catch (e) {
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: `Error: ${e}`,
      };
      setMessages((prev: Message[]) => [...prev, errorMessage]);
    } finally {
      setIsProcessing(false);
    }
  }

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100 flex flex-col">
      {/* Minimal Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-zinc-800">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center font-bold text-white text-sm">
            ⠎
          </div>
          <span className="font-semibold">SAL</span>
          <span className="text-xs text-zinc-500">Braille-Native AI</span>
        </div>
        
        <div className="flex items-center gap-4">
          {/* Network Status */}
          <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium ${
            networkStatus === "offline" 
              ? "bg-green-500/20 text-green-400" 
              : networkStatus === "online"
              ? "bg-red-500/20 text-red-400"
              : "bg-yellow-500/20 text-yellow-400"
          }`}>
            {networkStatus === "offline" ? (
              <>
                <WifiOff className="w-3 h-3" />
                <span>Secure</span>
              </>
            ) : networkStatus === "online" ? (
              <>
                <Wifi className="w-3 h-3" />
                <span>Network Detected - Disconnect to use SAL</span>
              </>
            ) : (
              <Loader2 className="w-3 h-3 animate-spin" />
            )}
          </div>

          {/* Settings */}
          <button
            onClick={() => setCurrentView(currentView === "settings" ? "chat" : "settings")}
            className={`p-2 rounded-lg transition-colors ${
              currentView === "settings" 
                ? "bg-zinc-800 text-white" 
                : "text-zinc-500 hover:text-white hover:bg-zinc-800"
            }`}
          >
            <Shield className="w-5 h-5" />
          </button>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex flex-col">
        {currentView === "onboarding" && (
          <OnboardingView
            whatsappDetected={whatsappDetected}
            isImporting={isImporting}
            onImport={handleImportWhatsApp}
            statusMessage={statusMessage}
          />
        )}

        {currentView === "chat" && (
          <ChatView
            messages={messages}
            inputValue={inputValue}
            setInputValue={setInputValue}
            onSend={handleSendMessage}
            isProcessing={isProcessing}
            networkStatus={networkStatus}
            vaultStats={vaultStats}
            suggestedQueries={suggestedQueries}
            lattice={lattice}
          />
        )}

        {currentView === "settings" && (
          <SettingsView
            federationStatus={federationStatus}
            onOptIn={handleOptIn}
            onOptOut={handleOptOut}
            vaultStats={vaultStats}
            onBack={() => setCurrentView("chat")}
          />
        )}
      </main>
    </div>
  );
}

function OnboardingView({
  whatsappDetected,
  isImporting,
  onImport,
  statusMessage,
}: {
  whatsappDetected: boolean | null;
  isImporting: boolean;
  onImport: () => void;
  statusMessage: string;
}) {
  return (
    <div className="flex-1 flex items-center justify-center p-8">
      <div className="max-w-md w-full">
        {/* Hero */}
        <div className="text-center mb-12">
          <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center mx-auto mb-6 shadow-lg shadow-violet-500/20 text-4xl">
            ⠎
          </div>
          <h1 className="text-3xl font-bold mb-3">Meet SAL</h1>
          <p className="text-zinc-400">
            The first AI to speak natively in Braille. 
            <span className="text-violet-400"> Born from your messages.</span>
          </p>
        </div>

        {/* Status Card */}
        <div className="bg-zinc-900 rounded-2xl p-6 mb-6 border border-zinc-800">
          {whatsappDetected === null ? (
            <div className="flex items-center gap-4">
              <Loader2 className="w-8 h-8 text-green-500 animate-spin" />
              <div>
                <p className="font-medium">Detecting WhatsApp...</p>
                <p className="text-sm text-zinc-500">Looking for your messages</p>
              </div>
            </div>
          ) : whatsappDetected ? (
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-green-500/20 flex items-center justify-center">
                <CheckCircle2 className="w-6 h-6 text-green-500" />
              </div>
              <div className="flex-1">
                <p className="font-medium text-green-400">WhatsApp Desktop Found!</p>
                <p className="text-sm text-zinc-500">Ready to import your messages</p>
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-yellow-500/20 flex items-center justify-center">
                <AlertTriangle className="w-6 h-6 text-yellow-500" />
              </div>
              <div className="flex-1">
                <p className="font-medium text-yellow-400">WhatsApp Desktop Not Found</p>
                <p className="text-sm text-zinc-500">Install WhatsApp Desktop to continue</p>
              </div>
            </div>
          )}
        </div>

        {/* Import Button */}
        <button
          onClick={onImport}
          disabled={!whatsappDetected || isImporting}
          className="w-full py-4 px-6 bg-gradient-to-r from-green-500 to-emerald-600 text-white font-semibold rounded-xl hover:from-green-600 hover:to-emerald-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-3 shadow-lg shadow-green-500/20"
        >
          {isImporting ? (
            <>
              <Loader2 className="w-5 h-5 animate-spin" />
              <span>{statusMessage || "Importing Messages..."}</span>
            </>
          ) : (
            <>
              <Zap className="w-5 h-5" />
              <span>Import My WhatsApp</span>
              <ArrowRight className="w-5 h-5" />
            </>
          )}
        </button>

        {/* Features */}
        <div className="mt-8 grid grid-cols-3 gap-4">
          <div className="text-center">
            <div className="w-10 h-10 rounded-lg bg-zinc-800 flex items-center justify-center mx-auto mb-2">
              <WifiOff className="w-5 h-5 text-green-500" />
            </div>
            <p className="text-xs text-zinc-500">Works Offline</p>
          </div>
          <div className="text-center">
            <div className="w-10 h-10 rounded-lg bg-zinc-800 flex items-center justify-center mx-auto mb-2">
              <Shield className="w-5 h-5 text-green-500" />
            </div>
            <p className="text-xs text-zinc-500">Private by Design</p>
          </div>
          <div className="text-center">
            <div className="w-10 h-10 rounded-lg bg-zinc-800 flex items-center justify-center mx-auto mb-2">
              <Smartphone className="w-5 h-5 text-green-500" />
            </div>
            <p className="text-xs text-zinc-500">Local AI</p>
          </div>
        </div>

        {/* Privacy Note */}
        <p className="text-xs text-zinc-600 text-center mt-8">
          Your messages never leave this device. We can't see them. No one can.
        </p>
      </div>
    </div>
  );
}

function ChatView({
  messages,
  inputValue,
  setInputValue,
  onSend,
  isProcessing,
  networkStatus,
  vaultStats,
  suggestedQueries,
  lattice,
}: {
  messages: Message[];
  inputValue: string;
  setInputValue: (value: string) => void;
  onSend: () => void;
  isProcessing: boolean;
  networkStatus: NetworkStatus;
  vaultStats: VaultStats;
  suggestedQueries: string[];
  lattice: LatticeSnapshot | null;
}) {
  const isLocked = networkStatus === "online";

  return (
    <div className="flex-1 flex flex-col max-w-3xl mx-auto w-full p-6">
      {/* Lattice - Key People */}
      {lattice && lattice.key_people.length > 0 && (
        <div className="mb-6 p-4 bg-gradient-to-r from-violet-500/10 to-purple-600/10 border border-violet-500/20 rounded-xl">
          <h3 className="text-sm font-medium text-violet-400 mb-3">⠎ SAL knows these people in your life:</h3>
          <div className="flex flex-wrap gap-2">
            {lattice.key_people.slice(0, 10).map((person, i) => (
              <span 
                key={i}
                className="px-3 py-1 bg-violet-500/20 text-violet-300 rounded-full text-sm"
                title={`${person.mentions} mentions, κ=${person.curvature.toFixed(2)}`}
              >
                {person.name}
                <span className="ml-1 text-violet-500 text-xs">({person.mentions})</span>
              </span>
            ))}
          </div>
          {lattice.relationships.length > 0 && (
            <div className="mt-3 pt-3 border-t border-violet-500/20">
              <p className="text-xs text-zinc-500">
                {lattice.total_atoms} meaning atoms • {lattice.total_edges} relationship edges
              </p>
            </div>
          )}
        </div>
      )}

      {/* Stats Bar */}
      <div className="flex items-center justify-between mb-6 pb-4 border-b border-zinc-800">
        <div className="flex items-center gap-6">
          <div>
            <p className="text-2xl font-bold text-green-400">{vaultStats.total_messages.toLocaleString()}</p>
            <p className="text-xs text-zinc-500">messages indexed</p>
          </div>
        </div>
        {isLocked && (
          <div className="flex items-center gap-2 px-3 py-2 bg-red-500/10 border border-red-500/20 rounded-lg">
            <Lock className="w-4 h-4 text-red-400" />
            <span className="text-sm text-red-400">Enable Airplane Mode to search</span>
          </div>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Network Warning Banner */}
        {isLocked && (
          <div className="p-4 bg-red-500/10 border border-red-500/30 rounded-xl mb-4">
            <div className="flex items-center gap-3">
              <Wifi className="w-6 h-6 text-red-400" />
              <div>
                <p className="font-medium text-red-400">Network Connection Detected</p>
                <p className="text-sm text-zinc-400">
                  SAL detected an active internet connection (WiFi or Ethernet). 
                  Disconnect all network cables and disable WiFi to protect your privacy.
                </p>
              </div>
            </div>
          </div>
        )}

        {messages.length === 0 && !isLocked && (
          <div className="h-full flex items-center justify-center">
            <div className="text-center max-w-sm">
              <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-violet-500/20 to-purple-600/20 flex items-center justify-center mx-auto mb-4 text-3xl">
                ⠎
              </div>
              <h3 className="text-lg font-medium mb-2">Ask SAL Anything</h3>
              <p className="text-sm text-zinc-500 mb-4">
                I've read your messages. I know your story.
              </p>
              <div className="space-y-2 text-left">
                {suggestedQueries.map((query, i) => (
                  <button 
                    key={i}
                    onClick={() => setInputValue(query)}
                    className="w-full p-3 text-left text-sm bg-zinc-900 hover:bg-zinc-800 rounded-lg border border-zinc-800 transition-colors"
                  >
                    "{query}"
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}
        
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[85%] p-4 rounded-2xl ${
                message.role === "user"
                  ? "bg-green-600 text-white"
                  : "bg-zinc-800 border border-zinc-700"
              }`}
            >
              <p className="whitespace-pre-wrap">{message.content}</p>
              {message.sources && message.sources.length > 0 && (
                <div className="mt-3 pt-3 border-t border-zinc-600/50">
                  <p className="text-xs text-zinc-400 mb-1">From your chats:</p>
                  {message.sources.map((source, i) => (
                    <p key={i} className="text-xs text-zinc-500 truncate">
                      {source}
                    </p>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}
        
        {isProcessing && (
          <div className="flex justify-start">
            <div className="bg-zinc-800 border border-zinc-700 p-4 rounded-2xl">
              <Loader2 className="w-5 h-5 animate-spin text-green-500" />
            </div>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="flex gap-3">
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && onSend()}
          placeholder={isLocked ? "Enable Airplane Mode to search..." : "Ask about your messages..."}
          disabled={isLocked || isProcessing}
          className="flex-1 bg-zinc-900 border border-zinc-800 rounded-xl px-4 py-3 focus:outline-none focus:border-green-500 disabled:opacity-50 disabled:cursor-not-allowed placeholder:text-zinc-600"
        />
        <button
          onClick={onSend}
          disabled={isLocked || isProcessing || !inputValue.trim()}
          className="px-5 py-3 bg-green-600 text-white rounded-xl hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Send className="w-5 h-5" />
        </button>
      </div>
    </div>
  );
}

function SettingsView({
  federationStatus,
  onOptIn,
  onOptOut,
  vaultStats,
  onBack,
}: {
  federationStatus: FederationStatus;
  onOptIn: () => void;
  onOptOut: () => void;
  vaultStats: VaultStats;
  onBack: () => void;
}) {
  return (
    <div className="flex-1 max-w-2xl mx-auto w-full p-6">
      <button
        onClick={onBack}
        className="text-zinc-500 hover:text-white mb-6 text-sm"
      >
        ← Back to Chat
      </button>

      <h2 className="text-2xl font-bold mb-2">Privacy Settings</h2>
      <p className="text-zinc-400 mb-8">Control your data and contribute to collective AI</p>

      {/* Vault Stats */}
      <div className="bg-zinc-900 rounded-xl p-5 mb-6 border border-zinc-800">
        <h3 className="font-medium mb-4">Your Vault</h3>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className="text-3xl font-bold text-green-400">{vaultStats.total_messages.toLocaleString()}</p>
            <p className="text-sm text-zinc-500">WhatsApp messages</p>
          </div>
          <div>
            <p className="text-3xl font-bold">{vaultStats.sources.length}</p>
            <p className="text-sm text-zinc-500">data sources</p>
          </div>
        </div>
      </div>

      {/* Federation */}
      <div className={`rounded-xl p-5 mb-6 border ${
        federationStatus.opted_in 
          ? "bg-green-500/10 border-green-500/30" 
          : "bg-zinc-900 border-zinc-800"
      }`}>
        <div className="flex items-start justify-between mb-4">
          <div>
            <h3 className="font-medium flex items-center gap-2">
              <Users className="w-5 h-5" />
              Collective Intelligence
            </h3>
            <p className="text-sm text-zinc-400 mt-1">
              Help improve AI for everyone with anonymized data
            </p>
          </div>
          <button
            onClick={federationStatus.opted_in ? onOptOut : onOptIn}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              federationStatus.opted_in
                ? "bg-zinc-800 text-zinc-300 hover:bg-zinc-700"
                : "bg-green-600 text-white hover:bg-green-700"
            }`}
          >
            {federationStatus.opted_in ? "Opt Out" : "Opt In"}
          </button>
        </div>

        <div className="space-y-2 text-sm text-zinc-500">
          <p className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-green-500" />
            Only anonymized embeddings shared - never raw messages
          </p>
          <p className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-green-500" />
            Differential privacy ensures your data can't be reconstructed
          </p>
          <p className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-green-500" />
            You can opt out anytime - we delete your contributions
          </p>
        </div>
      </div>

      {/* Privacy Info */}
      <div className="bg-zinc-900 rounded-xl p-5 border border-zinc-800">
        <h3 className="font-medium mb-3">How Your Data is Protected</h3>
        <ul className="space-y-3 text-sm text-zinc-400">
          <li className="flex items-start gap-3">
            <WifiOff className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
            <span>AI queries only work in Airplane Mode - we literally can't phone home</span>
          </li>
          <li className="flex items-start gap-3">
            <Shield className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
            <span>All processing happens on your device - messages never leave</span>
          </li>
          <li className="flex items-start gap-3">
            <Lock className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
            <span>SOC 2 compliant infrastructure for any synced data</span>
          </li>
        </ul>
      </div>
    </div>
  );
}

export default App;
