import { BrowserRouter, Routes, Route, NavLink } from "react-router-dom";
import DashboardPage from "./pages/DashboardPage";
import AdminPage from "./pages/AdminPage";
import ShareCardPage from "./pages/ShareCardPage";
import TickerShareCardPage from "./pages/TickerShareCardPage";
import StatusPage from "./pages/StatusPage";

function NavBar() {
  const cls = ({ isActive }: { isActive: boolean }) =>
    `text-sm px-3 py-1.5 rounded-lg transition-colors ${
      isActive
        ? "bg-violet-600 text-white"
        : "text-gray-400 hover:text-gray-200 hover:bg-gray-800"
    }`;
  return (
    <nav className="flex items-center gap-1 px-4 py-3 border-b border-gray-800 bg-gray-900">
      <span className="text-gray-100 font-semibold mr-4 text-sm">korucha-fund</span>
      <NavLink to="/" end className={cls}>ダッシュボード</NavLink>
      <NavLink to="/admin" className={cls}>取引管理</NavLink>
      <NavLink to="/status" className={cls}>ステータス</NavLink>
    </nav>
  );
}

function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-gray-900 text-gray-100">
      <NavBar />
      {children}
    </div>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout><DashboardPage /></Layout>} />
        <Route path="/admin" element={<Layout><AdminPage /></Layout>} />
        <Route path="/status" element={<Layout><StatusPage /></Layout>} />
        <Route path="/share/:id" element={<Layout><ShareCardPage /></Layout>} />
        <Route path="/ticker/:id" element={<Layout><TickerShareCardPage /></Layout>} />
        <Route path="*" element={<Layout><div className="p-8 text-gray-400">404 Not Found</div></Layout>} />
      </Routes>
    </BrowserRouter>
  );
}
