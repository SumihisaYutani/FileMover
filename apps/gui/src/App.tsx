import { Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import SetupPage from './pages/SetupPage';
import ScanResultsPage from './pages/ScanResultsPage';
import PlanPage from './pages/PlanPage';
import ExecutionPage from './pages/ExecutionPage';
import ResultsPage from './pages/ResultsPage';

function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<SetupPage />} />
        <Route path="/scan-results/:sessionId" element={<ScanResultsPage />} />
        <Route path="/plan/:sessionId" element={<PlanPage />} />
        <Route path="/execution/:sessionId" element={<ExecutionPage />} />
        <Route path="/results/:sessionId" element={<ResultsPage />} />
      </Routes>
    </Layout>
  );
}

export default App;