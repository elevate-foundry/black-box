import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { 
  Shield, 
  Wifi, 
  WifiOff, 
  MessageSquare, 
  Upload, 
  Search,
  Lock,
  Loader2,
  Send,
  Database,
  Brain,
  CheckCircle2,
  AlertTriangle
} from "lucide-react";

type NetworkStatus = "offline" | "online" | "checking";
type AppView = "dashboard" | "import" | "chat";
type ImportSource = "imessage" | "whatsapp" | "slack";

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

function App() {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>("checking");
  const [currentView, setCurrentView] = useState<AppView>("dashboard");
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isProcessing, setIsProcessing] = useState(false);
  const [vaultStats, setVaultStats] = useState<VaultStats>({
    total_messages: 0,
    sources: [],
    last_indexed: null,
  });
  const [importProgress, setImportProgress] = useState<number | null>(null);

  useEffect(() => {
    checkNetworkStatus();
    loadVaultStats();
    const interval = setInterval(checkNetworkStatus, 3000);
    return () => clearInterval(interval);
  }, []);

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

  async function handleImport(source: ImportSource) {
    setImportProgress(0);
    try {
      await invoke("import_messages", { source });
      setImportProgress(100);
      await loadVaultStats();
      setTimeout(() => {
        setImportProgress(null);
        setCurrentView("dashboard");
      }, 1500);
    } catch (e) {
      console.error("Import failed:", e);
      setImportProgress(null);
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

    setMessages((prev) => [...prev, userMessage]);
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

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (e) {
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: `Error: ${e}`,
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsProcessing(false);
    }
  }

  return (
    <div className="min-h-screen bg-vault-bg text-zinc-100 flex flex-col">
      {/* Status Bar */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-vault-border bg-vault-surface">
        <div className="flex items-center gap-3">
          <Shield className="w-8 h-8 text-vault-accent" />
          <h1 className="text-xl font-semibold tracking-tight">The Black Box</h1>
        </div>
        
        <div className="flex items-center gap-4">
          {/* Network Status Indicator */}
          <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-sm font-medium ${
            networkStatus === "offline" 
              ? "bg-vault-accent/20 text-vault-accent" 
              : networkStatus === "online"
              ? "bg-vault-danger/20 text-vault-danger"
              : "bg-vault-warning/20 text-vault-warning"
          }`}>
            {networkStatus === "offline" ? (
              <>
                <WifiOff className="w-4 h-4 status-indicator" />
                <span>Vault Secured</span>
              </>
            ) : networkStatus === "online" ? (
              <>
                <Wifi className="w-4 h-4" />
                <span>Network Detected</span>
              </>
            ) : (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Checking...</span>
              </>
            )}
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex">
        {/* Sidebar */}
        <nav className="w-64 border-r border-vault-border bg-vault-surface p-4 flex flex-col gap-2">
          <button
            onClick={() => setCurrentView("dashboard")}
            className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-colors ${
              currentView === "dashboard"
                ? "bg-vault-accent/20 text-vault-accent"
                : "hover:bg-vault-border text-zinc-400 hover:text-zinc-100"
            }`}
          >
            <Database className="w-5 h-5" />
            <span>Dashboard</span>
          </button>
          
          <button
            onClick={() => setCurrentView("import")}
            className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-colors ${
              currentView === "import"
                ? "bg-vault-accent/20 text-vault-accent"
                : "hover:bg-vault-border text-zinc-400 hover:text-zinc-100"
            }`}
          >
            <Upload className="w-5 h-5" />
            <span>Import Data</span>
          </button>
          
          <button
            onClick={() => setCurrentView("chat")}
            disabled={networkStatus === "online"}
            className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-colors ${
              currentView === "chat"
                ? "bg-vault-accent/20 text-vault-accent"
                : networkStatus === "online"
                ? "opacity-50 cursor-not-allowed text-zinc-600"
                : "hover:bg-vault-border text-zinc-400 hover:text-zinc-100"
            }`}
          >
            <MessageSquare className="w-5 h-5" />
            <span>Query Vault</span>
            {networkStatus === "online" && <Lock className="w-4 h-4 ml-auto" />}
          </button>

          <div className="mt-auto pt-4 border-t border-vault-border">
            <div className="text-xs text-zinc-500 space-y-1">
              <p>Messages indexed: <span className="text-zinc-300">{vaultStats.total_messages.toLocaleString()}</span></p>
              <p>Sources: <span className="text-zinc-300">{vaultStats.sources.length}</span></p>
            </div>
          </div>
        </nav>

        {/* Content Area */}
        <div className="flex-1 p-6">
          {currentView === "dashboard" && (
            <DashboardView 
              networkStatus={networkStatus} 
              vaultStats={vaultStats}
              onNavigate={setCurrentView}
            />
          )}
          
          {currentView === "import" && (
            <ImportView 
              onImport={handleImport}
              progress={importProgress}
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
            />
          )}
        </div>
      </main>
    </div>
  );
}

function DashboardView({ 
  networkStatus, 
  vaultStats,
  onNavigate 
}: { 
  networkStatus: NetworkStatus;
  vaultStats: VaultStats;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <div className="fade-in max-w-4xl">
      <h2 className="text-2xl font-semibold mb-6">Your Digital Vault</h2>
      
      {/* Security Status Card */}
      <div className={`p-6 rounded-xl border mb-6 ${
        networkStatus === "offline"
          ? "bg-vault-accent/10 border-vault-accent/30"
          : "bg-vault-danger/10 border-vault-danger/30"
      }`}>
        <div className="flex items-start gap-4">
          {networkStatus === "offline" ? (
            <CheckCircle2 className="w-8 h-8 text-vault-accent flex-shrink-0" />
          ) : (
            <AlertTriangle className="w-8 h-8 text-vault-danger flex-shrink-0" />
          )}
          <div>
            <h3 className={`text-lg font-semibold ${
              networkStatus === "offline" ? "text-vault-accent" : "text-vault-danger"
            }`}>
              {networkStatus === "offline" 
                ? "Vault is Secured" 
                : "Network Connection Detected"}
            </h3>
            <p className="text-zinc-400 mt-1">
              {networkStatus === "offline"
                ? "Your data is completely isolated. No bytes can leave this device."
                : "For your security, please enable Airplane Mode to access the vault. Your memories deserve military-grade privacy."}
            </p>
          </div>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <div className="bg-vault-surface border border-vault-border rounded-xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <MessageSquare className="w-5 h-5 text-vault-accent" />
            <span className="text-zinc-400 text-sm">Messages</span>
          </div>
          <p className="text-3xl font-semibold">{vaultStats.total_messages.toLocaleString()}</p>
        </div>
        
        <div className="bg-vault-surface border border-vault-border rounded-xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <Database className="w-5 h-5 text-vault-accent" />
            <span className="text-zinc-400 text-sm">Sources</span>
          </div>
          <p className="text-3xl font-semibold">{vaultStats.sources.length}</p>
        </div>
        
        <div className="bg-vault-surface border border-vault-border rounded-xl p-5">
          <div className="flex items-center gap-3 mb-3">
            <Brain className="w-5 h-5 text-vault-accent" />
            <span className="text-zinc-400 text-sm">Model</span>
          </div>
          <p className="text-lg font-semibold">Phi-3 Mini</p>
          <p className="text-xs text-zinc-500">3.8B params</p>
        </div>
      </div>

      {/* Quick Actions */}
      <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
      <div className="grid grid-cols-2 gap-4">
        <button
          onClick={() => onNavigate("import")}
          className="flex items-center gap-4 p-5 bg-vault-surface border border-vault-border rounded-xl hover:border-vault-accent/50 transition-colors text-left"
        >
          <Upload className="w-8 h-8 text-vault-accent" />
          <div>
            <p className="font-medium">Import Messages</p>
            <p className="text-sm text-zinc-500">Add iMessage, WhatsApp, or Slack data</p>
          </div>
        </button>
        
        <button
          onClick={() => onNavigate("chat")}
          disabled={networkStatus === "online"}
          className={`flex items-center gap-4 p-5 bg-vault-surface border border-vault-border rounded-xl transition-colors text-left ${
            networkStatus === "online"
              ? "opacity-50 cursor-not-allowed"
              : "hover:border-vault-accent/50"
          }`}
        >
          <Search className="w-8 h-8 text-vault-accent" />
          <div>
            <p className="font-medium">Query Your Memory</p>
            <p className="text-sm text-zinc-500">
              {networkStatus === "online" 
                ? "Enable Airplane Mode first" 
                : "Ask anything about your past"}
            </p>
          </div>
        </button>
      </div>
    </div>
  );
}

function ImportView({ 
  onImport, 
  progress 
}: { 
  onImport: (source: ImportSource) => void;
  progress: number | null;
}) {
  const sources: { id: ImportSource; name: string; description: string; icon: string }[] = [
    {
      id: "imessage",
      name: "iMessage",
      description: "Import from your local chat.db (requires Full Disk Access)",
      icon: "💬",
    },
    {
      id: "whatsapp",
      name: "WhatsApp",
      description: "Import from WhatsApp chat export (.txt files)",
      icon: "📱",
    },
    {
      id: "slack",
      name: "Slack",
      description: "Import from Slack workspace export (JSON)",
      icon: "💼",
    },
  ];

  return (
    <div className="fade-in max-w-2xl">
      <h2 className="text-2xl font-semibold mb-2">Import Your Data</h2>
      <p className="text-zinc-400 mb-8">
        Digitize your memories. All processing happens locally on your device.
      </p>

      {progress !== null && (
        <div className="mb-8 p-4 bg-vault-surface border border-vault-border rounded-xl">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium">Digitizing your memory...</span>
            <span className="text-sm text-vault-accent">{progress}%</span>
          </div>
          <div className="h-2 bg-vault-border rounded-full overflow-hidden">
            <div 
              className="h-full bg-vault-accent transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      <div className="space-y-4">
        {sources.map((source) => (
          <button
            key={source.id}
            onClick={() => onImport(source.id)}
            disabled={progress !== null}
            className="w-full flex items-center gap-4 p-5 bg-vault-surface border border-vault-border rounded-xl hover:border-vault-accent/50 transition-colors text-left disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <span className="text-3xl">{source.icon}</span>
            <div className="flex-1">
              <p className="font-medium">{source.name}</p>
              <p className="text-sm text-zinc-500">{source.description}</p>
            </div>
            <Upload className="w-5 h-5 text-zinc-500" />
          </button>
        ))}
      </div>

      <div className="mt-8 p-4 bg-vault-surface/50 border border-vault-border rounded-xl">
        <p className="text-sm text-zinc-500">
          <strong className="text-zinc-300">Privacy Note:</strong> Your data never leaves this device. 
          The Black Box creates a local vector index for semantic search. No cloud. No sync. No leaks.
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
}: {
  messages: Message[];
  inputValue: string;
  setInputValue: (value: string) => void;
  onSend: () => void;
  isProcessing: boolean;
  networkStatus: NetworkStatus;
}) {
  return (
    <div className="fade-in h-full flex flex-col">
      <div className="mb-4">
        <h2 className="text-2xl font-semibold">Query Your Vault</h2>
        <p className="text-zinc-400">Ask anything about your past conversations</p>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {messages.length === 0 && (
          <div className="h-full flex items-center justify-center">
            <div className="text-center text-zinc-500">
              <Search className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>Ask a question to search your memory</p>
              <p className="text-sm mt-2">
                Try: "When did I last talk to John about the project?"
              </p>
            </div>
          </div>
        )}
        
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[80%] p-4 rounded-xl ${
                message.role === "user"
                  ? "bg-vault-accent text-white"
                  : "bg-vault-surface border border-vault-border"
              }`}
            >
              <p className="whitespace-pre-wrap">{message.content}</p>
              {message.sources && message.sources.length > 0 && (
                <div className="mt-3 pt-3 border-t border-vault-border/50">
                  <p className="text-xs text-zinc-400 mb-1">Sources:</p>
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
            <div className="bg-vault-surface border border-vault-border p-4 rounded-xl">
              <Loader2 className="w-5 h-5 animate-spin text-vault-accent" />
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
          placeholder={
            networkStatus === "online"
              ? "Enable Airplane Mode to query..."
              : "Ask about your memories..."
          }
          disabled={networkStatus === "online" || isProcessing}
          className="flex-1 bg-vault-surface border border-vault-border rounded-xl px-4 py-3 focus:outline-none focus:border-vault-accent disabled:opacity-50 disabled:cursor-not-allowed"
        />
        <button
          onClick={onSend}
          disabled={networkStatus === "online" || isProcessing || !inputValue.trim()}
          className="px-5 py-3 bg-vault-accent text-white rounded-xl hover:bg-vault-accent/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Send className="w-5 h-5" />
        </button>
      </div>
    </div>
  );
}

export default App;
