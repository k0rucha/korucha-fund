// Register the Chart.js pieces used by the dashboard charts. Imported for its
// side effect by the chart components (module runs once, registration cached).
import {
  ArcElement,
  CategoryScale,
  Chart,
  DoughnutController,
  Filler,
  Legend,
  LineController,
  LineElement,
  LinearScale,
  PointElement,
  Tooltip,
} from "chart.js";

Chart.register(
  ArcElement,
  LineElement,
  PointElement,
  LineController,
  DoughnutController,
  CategoryScale,
  LinearScale,
  Filler,
  Tooltip,
  Legend,
);
