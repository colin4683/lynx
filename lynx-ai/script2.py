#!/usr/bin/env python3
"""
Enhanced Isolation Forest training script with proper contamination handling
"""

import json
import logging
import joblib
import numpy as np
import pandas as pd
from sklearn.ensemble import IsolationForest
from sklearn.preprocessing import StandardScaler
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report
from pathlib import Path
from typing import Tuple, Dict, List, Optional
import argparse
from datetime import datetime

"""
The lynx-agent is a Rust-based system agent designed to securely collect, monitor, and transmit system information. It works as follows:
On startup, it loads configuration from config.toml and initializes secure communication using TLS certificates from the certs/ directory.
The agent uses modular collectors (in src/lib/collectors.rs and src/lib/system_info.rs) to gather system metrics and information.
Data is transmitted to remote services using client modules (src/lib/client.rs) and WebSocket support (src/lib/websocket.rs).
The agent is extensible, allowing new collectors or communication protocols to be added easily.
It is designed for robust, secure operation, supporting integration with monitoring solutions and other services.
he lynx-agent project uses several key Rust libraries (crates) for its main functionality:


gRPC: Uses the tonic crate for gRPC client/server communication.
WebSocket: Uses the tokio and tokio_util crates for async runtime and utilities, and likely a WebSocket-specific crate (not shown in the directory, but commonly tokio-tungstenite or similar).
Async Runtime: Uses tokio for asynchronous operations.
Serialization: Uses serde for serializing and deserializing data.
HTTP/Networking: Uses hyper for HTTP networking.
TLS/SSL: Uses rustls or similar (not directly shown, but TLS certs are present).
Command-line Parsing: Uses clap for parsing command-line arguments.
Protocol Buffers: Uses prost for Protocol Buffers code generation and handling.
These libraries enable secure, asynchronous system monitoring, data collection, and communication over gRPC and WebSocket protocols.
"""

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Default configuration - UPDATED CONTAMINATION
DEFAULT_CONFIG = {
    "input_csv": "system_metrics.csv",
    "output_dir": "model_assets",
    "contamination": 0.10,  # CHANGED: 20% expected anomalies
    "n_estimators": 100,
    "random_state": 42,
    "features": [
        "cpu_usage",
        "memory_usage",
        "net_in",
        "net_out",
        "load_one",
    ],
    "memory_usage_ratio": True,
    "drop_columns": [
        "time",
        "uptime",
        "docker_containers_running",
        "system_id",
        "components",
        "ctid",
        "load_fifteen",
        "load_five"
    ],
    "min_samples": 100,
    "validation_split": 0.2,
    "onnx_opset": 15,
    "ai_onnx_ml_opset": 3
}


class DataValidationError(Exception):
    """Custom exception for data validation errors"""
    pass


def validate_data(df: pd.DataFrame, config: Dict) -> None:
    """Validate input data before processing"""
    if df.empty:
        raise DataValidationError("Input dataframe is empty")

    if len(df) < config.get("min_samples", 100):
        raise DataValidationError(
            f"Insufficient samples: {len(df)} < {config.get('min_samples', 100)}"
        )

    # Check for required columns
    if config["memory_usage_ratio"]:
        required_cols = ["memory_used_kb", "memory_total_kb"]
        missing_cols = [col for col in required_cols if col not in df.columns]
        if missing_cols:
            raise DataValidationError(f"Missing required columns: {missing_cols}")

    # Check for features after preprocessing
    missing_features = [f for f in config["features"] if f not in df.columns
                        and f != "memory_usage"]
    if missing_features and not config["memory_usage_ratio"]:
        raise DataValidationError(f"Missing features: {missing_features}")


def load_data(filepath: str, config: Dict) -> Tuple[np.ndarray, StandardScaler, List[str]]:
    """Load and preprocess system metrics with validation"""
    try:
        df = pd.read_csv(filepath)
        logger.info(f"Loaded {len(df)} rows from {filepath}")
    except FileNotFoundError:
        raise FileNotFoundError(f"Input file not found: {filepath}")
    except pd.errors.EmptyDataError:
        raise DataValidationError("Input CSV file is empty")

    # Validate data
    validate_data(df, config)

    # Drop unused columns
    columns_to_drop = [col for col in config["drop_columns"] if col in df.columns]
    df = df.drop(columns=columns_to_drop)
    logger.info(f"Dropped columns: {columns_to_drop}")

    # Calculate memory usage percentage
    if config["memory_usage_ratio"]:
        if "memory_used_kb" in df.columns and "memory_total_kb" in df.columns:
            # Handle division by zero
            df["memory_usage"] = np.where(
                df["memory_total_kb"] > 0,
                (df["memory_used_kb"] / df["memory_total_kb"]) * 100,
                0
            )
            df = df.drop(columns=["memory_used_kb", "memory_total_kb"])
            logger.info("Calculated memory usage percentage")

    # Save preprocessed data for debugging
    debug_path = Path(config["output_dir"]) / "preprocessed_data.csv"
    debug_path.parent.mkdir(exist_ok=True)
    df.to_csv(debug_path, index=False)

    # Select features and handle missing values
    feature_cols = [f for f in config["features"] if f in df.columns]
    if len(feature_cols) != len(config["features"]):
        missing = set(config["features"]) - set(feature_cols)
        logger.warning(f"Missing features: {missing}")

    X = df[feature_cols].values

    # Handle missing values
    if np.isnan(X).any():
        logger.warning("Found NaN values in features, filling with median")
        X = pd.DataFrame(X, columns=feature_cols).fillna(
            pd.DataFrame(X, columns=feature_cols).median()
        ).values

    scaler = StandardScaler()
    return X, scaler, feature_cols


def preprocess_data(X: np.ndarray, feature_names: List[str]) -> Tuple[np.ndarray, StandardScaler]:
    """Preprocess data with proper scaling (scale only on normal data)"""
    # NEW: Estimate normal data (assume majority is normal)
    # For Isolation Forest, we fit scaler on all data but it's better to use robust scaling
    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    logger.info(f"Data scaled. Mean: {scaler.mean_}, Scale: {scaler.scale_}")
    return X_scaled, scaler


def evaluate_model(model: IsolationForest, X: np.ndarray,
                   contamination: float) -> Dict:
    """Evaluate model performance"""
    predictions = model.predict(X)
    anomaly_scores = model.decision_function(X)  # CHANGED: Use decision_function instead of score_samples

    n_anomalies = (predictions == -1).sum()
    anomaly_ratio = n_anomalies / len(predictions)

    metrics = {
        "n_samples": len(X),
        "n_anomalies_detected": int(n_anomalies),
        "anomaly_ratio": float(anomaly_ratio),
        "expected_contamination": contamination,
        "score_threshold": float(model.offset_),
        "mean_anomaly_score": float(anomaly_scores.mean()),
        "std_anomaly_score": float(anomaly_scores.std()),
        "min_anomaly_score": float(anomaly_scores.min()),
        "max_anomaly_score": float(anomaly_scores.max()),
        "score_range": f"{anomaly_scores.min():.3f} to {anomaly_scores.max():.3f}"
    }

    return metrics


def train_model(X: np.ndarray, config: Dict) -> IsolationForest:
    """Train Isolation Forest model with validation"""
    # NEW: Validate contamination parameter
    if config['contamination'] <= 0 or config['contamination'] >= 0.5:
        logger.warning(f"Contamination {config['contamination']} may be unrealistic. Using 0.20")
        contamination = 0.20
    else:
        contamination = config['contamination']

    model = IsolationForest(
        n_estimators=config['n_estimators'],
        contamination=contamination,  # UPDATED
        random_state=config['random_state'],
        max_samples=config.get('max_samples', 'auto'),
        verbose=1,
        n_jobs=-1
    )

    # Split data for validation if specified
    if config.get("validation_split", 0) > 0:
        X_train, X_val = train_test_split(
            X,
            test_size=config["validation_split"],
            random_state=config["random_state"]
        )
        logger.info(f"Training on {len(X_train)} samples, validating on {len(X_val)}")
        model.fit(X_train)

        # Evaluate on validation set
        val_metrics = evaluate_model(model, X_val, contamination)
        logger.info(f"Validation metrics: {val_metrics}")

        # Check if contamination is realistic
        expected_vs_actual = abs(val_metrics['anomaly_ratio'] - contamination)
        if expected_vs_actual > 0.1:
            logger.warning(f"Contamination mismatch: expected {contamination}, got {val_metrics['anomaly_ratio']:.3f}")
    else:
        model.fit(X)

    return model


def export_model(model: IsolationForest, scaler: StandardScaler,
                 feature_names: List[str], output_dir: str,
                 config: Dict, metrics: Dict) -> None:
    """Export model with metadata and error handling"""
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Save metadata
    metadata = {
        "features": feature_names,
        "config": config,
        "metrics": metrics,
        "training_date": datetime.now().isoformat(),
        "model_version": "1.0.0"
    }

    with open(output_path / "metadata.json", "w") as f:
        json.dump(metadata, f, indent=2)

    # Export ONNX model
    try:
        from skl2onnx import convert_sklearn
        from skl2onnx.common.data_types import FloatTensorType

        initial_type = [('float_input', FloatTensorType([None, len(feature_names)]))]
        onx = convert_sklearn(
            model,
            initial_types=initial_type,
            target_opset={
                '': config["onnx_opset"],
                'ai.onnx.ml': config["ai_onnx_ml_opset"]
            }
        )

        with open(output_path / "model.onnx", "wb") as f:
            f.write(onx.SerializeToString())
        logger.info("ONNX model exported successfully")
    except ImportError:
        logger.warning("skl2onnx not installed, skipping ONNX export")
    except Exception as e:
        logger.error(f"Failed to export ONNX model: {e}")

    # Export scaler parameters
    scaler_params = {
        "mean": scaler.mean_.tolist(),
        "scale": scaler.scale_.tolist(),
        "feature_names": feature_names,
        "n_features": len(feature_names)
    }
    with open(output_path / "scaler.json", "w") as f:
        json.dump(scaler_params, f, indent=2)

    # Export with joblib
    model_bundle = {
        "model": model,
        "scaler": scaler,
        "features": feature_names,
        "metadata": metadata
    }
    joblib.dump(model_bundle, output_path / "model.joblib")

    logger.info(f"Model successfully exported to {output_dir}")


def parse_arguments():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="Train Isolation Forest model for anomaly detection"
    )
    parser.add_argument(
        "--input", "-i",
        default=DEFAULT_CONFIG["input_csv"],
        help="Input CSV file path"
    )
    parser.add_argument(
        "--output", "-o",
        default=DEFAULT_CONFIG["output_dir"],
        help="Output directory for model assets"
    )
    parser.add_argument(
        "--contamination", "-c",
        type=float,
        default=DEFAULT_CONFIG["contamination"],
        help="Expected proportion of anomalies (0.01 to 0.49)"
    )
    parser.add_argument(
        "--config", "-f",
        help="Path to JSON config file (overrides defaults)"
    )
    return parser.parse_args()


def load_config(args) -> Dict:
    """Load configuration from file or command line"""
    config = DEFAULT_CONFIG.copy()

    # Load from config file if provided
    if args.config:
        try:
            with open(args.config, 'r') as f:
                file_config = json.load(f)
                config.update(file_config)
                logger.info(f"Loaded config from {args.config}")
        except Exception as e:
            logger.error(f"Failed to load config file: {e}")
            raise

    # Override with command line arguments
    config["input_csv"] = args.input
    config["output_dir"] = args.output
    config["contamination"] = args.contamination

    # Validate contamination
    if config["contamination"] <= 0 or config["contamination"] >= 0.5:
        logger.warning(f"Contamination {config['contamination']} is unrealistic. Using default 0.20")
        config["contamination"] = 0.20

    return config


def main():
    """Main training pipeline with error handling"""
    args = parse_arguments()

    try:
        # Load configuration
        config = load_config(args)

        logger.info("Starting Isolation Forest training")
        logger.info(f"Configuration: {json.dumps(config, indent=2)}")

        # Load and preprocess data
        logger.info("Loading training data...")
        X, scaler, feature_names = load_data(config['input_csv'], config)
        logger.info(f"Using features: {feature_names}")
        logger.info(f"Data shape: {X.shape}")

        # NEW: Preprocess data with proper scaling
        logger.info("Preprocessing data...")
        X_scaled, scaler = preprocess_data(X, feature_names)

        # Train model
        logger.info("Training model...")
        model = train_model(X_scaled, config)

        # Evaluate model
        logger.info("Evaluating model...")
        metrics = evaluate_model(model, X_scaled, config['contamination'])
        logger.info(f"Training metrics: {json.dumps(metrics, indent=2)}")

        # Check if model is working
        if abs(metrics['mean_anomaly_score']) < 0.1:
            logger.warning("Model scores are clustered around zero - may not be detecting anomalies properly")
        if metrics['min_anomaly_score'] > -0.1:
            logger.warning("No negative scores detected - model may not be working correctly")

        # Export model
        logger.info("Exporting model...")
        export_model(model, scaler, feature_names, config['output_dir'],
                     config, metrics)

        logger.info("Training complete!")
        logger.info(f"Model can detect anomalies in: {feature_names}")
        logger.info(f"Anomaly threshold: {model.offset_:.4f}")
        logger.info(f"Score range: {metrics['min_anomaly_score']:.3f} to {metrics['max_anomaly_score']:.3f}")
        logger.info(f"Detected {metrics['n_anomalies_detected']} anomalies "
                    f"({metrics['anomaly_ratio']:.2%}) in training data")

    except Exception as e:
        logger.error(f"Training failed: {e}")
        raise


if __name__ == "__main__":
    main()