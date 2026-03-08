import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect } from 'react';
import { Navbar } from './components/Navbar';
import { LandingPage } from './pages/LandingPage';
import { LoginPage } from './pages/LoginPage';
import { SignupPage } from './pages/SignupPage';
import { SolvePage } from './pages/SolvePage';
import { HistoryPage } from './pages/HistoryPage';
import { SettingsPage } from './pages/SettingsPage';
import { AggregatePage } from './pages/AggregatePage';
import { SolveResultPage } from './pages/SolveResultPage';
import { AggregateResultPage } from './pages/AggregateResultPage';
import { useAuthStore } from './store/authStore';
import { useWebSocket } from './hooks/useWebSocket';

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAuthStore();
  if (!isAuthenticated) return <Navigate to="/login" />;
  return <>{children}</>;
}

export function App() {
  const { loadFromStorage } = useAuthStore();

  useEffect(() => {
    loadFromStorage();
  }, [loadFromStorage]);

  // Connect WebSocket for real-time notifications
  useWebSocket();

  return (
    <BrowserRouter>
      <Navbar />
      <Routes>
        <Route path="/" element={<LandingPage />} />
        <Route path="/login" element={<LoginPage />} />
        <Route path="/signup" element={<SignupPage />} />
        <Route path="/app/solve" element={<ProtectedRoute><SolvePage /></ProtectedRoute>} />
        <Route path="/app/solve/:id" element={<ProtectedRoute><SolveResultPage /></ProtectedRoute>} />
        <Route path="/app/aggregate" element={<ProtectedRoute><AggregatePage /></ProtectedRoute>} />
        <Route path="/app/aggregate/:id" element={<ProtectedRoute><AggregateResultPage /></ProtectedRoute>} />
        <Route path="/app/history" element={<ProtectedRoute><HistoryPage /></ProtectedRoute>} />
        <Route path="/app/settings" element={<ProtectedRoute><SettingsPage /></ProtectedRoute>} />
      </Routes>
    </BrowserRouter>
  );
}
