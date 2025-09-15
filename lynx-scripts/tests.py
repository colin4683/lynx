#!/usr/bin/env python3
"""
Enhanced Isolation Forest test script with visual diagnostics
"""

import json
import joblib
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
from sklearn.metrics import confusion_matrix, ConfusionMatrixDisplay

# Configuration
MODEL_DIR = "model_assets"
PLOT_DIR = "test_plots"
Path(PLOT_DIR).mkdir(exist_ok=True)

# Test cases - normal and anomalous scenarios
TEST_CASES = [
    {
        "name": "Normal Operation",
        "data": {
            "cpu_usage": 25,
            "memory_usage": 10,
            "net_in": 2.0,
            "net_out": 2.0,
            "load_one": 0.5,
        },
        "should_be_anomaly": False
    },
    {
        "name": "High CPU Usage",
        "data": {
            "cpu_usage": 96.5,
            "memory_usage": 10.0,
            "net_in": 500.0,
            "net_out": 250.0,
            "load_one": 5.0,
        },
        "should_be_anomaly": True
    },
    {
        "name": "Memory Leak",
        "data": {
            "cpu_usage": 30.0,
            "memory_usage": 98.9,
            "net_in": 100.0,
            "net_out": 100.0,
            "load_one": 1.0,
        },
        "should_be_anomaly": True
    },
    {
        "name": "Network Storm",
        "data": {
            "cpu_usage": 60.0,
            "memory_usage": 70.0,
            "net_in": 5000.0,
            "net_out": 3000.0,
            "load_one": 10.0,
        },
        "should_be_anomaly": True
    }
]

class ModelTester:
    def __init__(self):
        self.model = None
        self.scaler = None
        self.feature_names = None
        self.test_results = []
        
    def load_resources(self):
        """Load model and scaler"""
        try:
            with open(Path(MODEL_DIR) / "scaler.json") as f:
                self.scaler = json.load(f)
            with open(Path(MODEL_DIR) / "features.json") as f:
                features = json.load(f)
                self.feature_names = features["features"]
            self.model = joblib.load(Path(MODEL_DIR) / "model.joblib")
            return True
        except Exception as e:
            print(f"❌ Failed to load resources: {str(e)}")
            return False
    
    def prepare_input(self, test_data):
        """Prepare test input"""
        try:
            x_test = np.array([[test_data[feat] for feat in self.feature_names]])
            x_normalized = (x_test - self.scaler["mean"]) / self.scaler["scale"]
            return x_normalized
        except Exception as e:
            print(f"❌ Input preparation failed: {str(e)}")
            return None
    
    def run_test_case(self, test_case):
        """Execute a single test case"""
        print(f"\n🧪 Testing: {test_case['name']}")
        print("Input values:")
        for k, v in test_case['data'].items():
            print(f"- {k}: {v}")
        
        x_test = self.prepare_input(test_case['data'])
        if x_test is None:
            return False
        
        # Get predictions
        pred = self.model["model"].decision_function(x_test)[0]
        label = self.model["model"].predict(x_test)[0]
        anomaly_score = 1 / (1 + np.exp(-pred))
        
        # Determine if test passed
        is_anomaly = label == -1
        test_passed = is_anomaly == test_case['should_be_anomaly']
        
        # Store results for visualization
        self.test_results.append({
            "name": test_case["name"],
            "score": pred,
            "probability": anomaly_score,
            "predicted": is_anomaly,
            "expected": test_case["should_be_anomaly"],
            "passed": test_passed,
            "features": test_case["data"]
        })
        
        # Print results
        status = "PASSED" if test_passed else "FAILED"
        emoji = "✅" if test_passed else "❌"
        print(f"\nResults:")
        print(f"- Anomaly score: {pred:.3f}")
        print(f"- Probability: {anomaly_score:.3f}")
        print(f"- Prediction: {'ANOMALY' if is_anomaly else 'NORMAL'}")
        print(f"- Label: {label}")
        print(f"- Expected: {'ANOMALY' if test_case['should_be_anomaly'] else 'NORMAL'}")
        print(f"{emoji} Test {status}")
        
        return test_passed
    
    def generate_visualizations(self):
        """Create diagnostic plots"""
        # 1. Score Distribution Plot
        plt.figure(figsize=(10, 6))
        scores = [r["score"] for r in self.test_results]
        categories = [r["name"] for r in self.test_results]
        colors = ['red' if r["expected"] else 'green' for r in self.test_results]
        
        plt.bar(categories, scores, color=colors)
        plt.axhline(y=self.model["model"].offset_, color='blue', linestyle='--', label='Decision Threshold')
        plt.title("Anomaly Scores by Test Case")
        plt.ylabel("Anomaly Score")
        plt.xticks(rotation=45)
        plt.legend()
        plt.tight_layout()
        plt.savefig(f"{PLOT_DIR}/anomaly_scores.png")
        plt.close()
        
        # 2. Feature Correlation Heatmap
        feature_data = np.array([[r["features"][f] for f in self.feature_names] for r in self.test_results])
        plt.figure(figsize=(10, 8))
        sns.heatmap(pd.DataFrame(feature_data, columns=self.feature_names).corr(),
                    annot=True, cmap='coolwarm', vmin=-1, vmax=1)
        plt.title("Feature Correlation in Test Cases")
        plt.tight_layout()
        plt.savefig(f"{PLOT_DIR}/feature_correlation.png")
        plt.close()
        
        # 3. Confusion Matrix
        y_true = [r["expected"] for r in self.test_results]
        y_pred = [r["predicted"] for r in self.test_results]
        cm = confusion_matrix(y_true, y_pred)
        disp = ConfusionMatrixDisplay(confusion_matrix=cm, display_labels=['Normal', 'Anomaly'])
        disp.plot(cmap='Blues')
        plt.title("Confusion Matrix")
        plt.savefig(f"{PLOT_DIR}/confusion_matrix.png")
        plt.close()
        
        # 4. Parallel Coordinates Plot
        plt.figure(figsize=(12, 6))
        pd.plotting.parallel_coordinates(
            pd.DataFrame([{**r["features"], "Status": "Anomaly" if r["expected"] else "Normal"} 
                         for r in self.test_results]),
            "Status",
            color=("#FF0000", "#00FF00")
        )
        plt.title("Feature Values by Test Case")
        plt.xticks(rotation=45)
        plt.tight_layout()
        plt.savefig(f"{PLOT_DIR}/parallel_coordinates.png")
        plt.close()

def main():
    tester = ModelTester()
    
    print("🚀 Starting Isolation Forest Model Tests")
    print("="*50)
    
    if not tester.load_resources():
        print("❌ Aborting tests due to load failure")
        return
    
    total_tests = len(TEST_CASES)
    passed_tests = 0
    
    for test_case in TEST_CASES:
        if tester.run_test_case(test_case):
            passed_tests += 1
    
    # Generate visual diagnostics
    tester.generate_visualizations()
    
    print("\n" + "="*50)
    print(f"📊 Test Summary: {passed_tests}/{total_tests} passed")
    print(f"🔧 Model threshold: {tester.model['model'].offset_:.3f}")
    print(f"📈 Visualizations saved to {PLOT_DIR}/ directory")
    
    if passed_tests == total_tests:
        print("🎉 All tests passed successfully!")
    else:
        print(f"⚠️  {total_tests - passed_tests} tests failed")

if __name__ == "__main__":
    main()
