#!/usr/bin/env python3
"""
Enhanced Isolation Forest training script with improved validation,
robust scaling, and better model evaluation for system metrics anomaly detection.
"""

import json
import logging
import joblib
import numpy as np
import pandas as pd
import hashlib
import psutil
from sklearn.ensemble import IsolationForest
from sklearn.preprocessing import StandardScaler, RobustScaler
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report
from pathlib import Path
from typing import Tuple, Dict, List, Optional, Union
import argparse
from datetime import datetime
import warnings

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Suppress sklearn warnings for cleaner output
warnings.filterwarnings('ignore', category=UserWarning, module='sklearn')

# Enhanced default configuration
DEFAULT_CONFIG = {
    "input_csv": "system_metrics.csv",
    "output_dir": "model_assets",
    "contamination": 0.10,
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
    "ai_onnx_ml_opset": 3,
    "robust_scaling": True,  # New: Use RobustScaler by default
    "max_samples": "auto",
    "bootstrap": False,
    "feature_selection": True  # New: Enable intelligent feature selection
}


class DataValidationError(Exception):
    """Custom exception for data validation errors"""
    pass


class ModelExportError(Exception):
    """Custom exception for model export errors"""
    pass


def log_memory_usage(stage: str) -> None:
    """Log current memory usage"""
    try:
        process = psutil.Process()
        memory_mb = process.memory_info().rss / 1024 / 1024
        logger.info(f"Memory usage at {stage}: {memory_mb:.1f} MB")
    except Exception:
        pass  # Don't fail if psutil is unavailable


def validate_data(df: pd.DataFrame, config: Dict) -> None:
    """Enhanced data validation with better error messages"""
    if df.empty:
        raise DataValidationError("Input dataframe is empty")

    if len(df) < config.get("min_samples", 100):
        raise DataValidationError(
            f"Insufficient samples: {len(df)} < {config.get('min_samples', 100)}. "
            f"Need at least {config.get('min_samples', 100)} samples for reliable training."
        )

    # Check data quality
    total_rows = len(df)
    null_counts = df.isnull().sum()
    high_null_cols = null_counts[null_counts > total_rows * 0.5].index.tolist()

    if high_null_cols:
        logger.warning(f"Columns with >50% missing data: {high_null_cols}")

    # Check for required columns for memory calculation
    if config["memory_usage_ratio"]:
        required_cols = ["memory_used_kb", "memory_total_kb"]
        missing_cols = [col for col in required_cols if col not in df.columns]
        if missing_cols:
            raise DataValidationError(
                f"Missing required columns for memory calculation: {missing_cols}"
            )

    logger.info(f"Data validation passed: {len(df)} samples, {len(df.columns)} features")


def validate_features(df: pd.DataFrame, config: Dict) -> List[str]:
    """Enhanced feature validation with intelligent selection"""
    available_features = []
    missing_features = []

    # Check base features
    for feature in config["features"]:
        if feature == "memory_usage" and config["memory_usage_ratio"]:
            if "memory_used_kb" in df.columns and "memory_total_kb" in df.columns:
                available_features.append(feature)
            else:
                missing_features.append(f"{feature} (requires memory_used_kb, memory_total_kb)")
        elif feature in df.columns:
            # Check if feature has sufficient variance
            if df[feature].nunique() > 1 and df[feature].std() > 0:
                available_features.append(feature)
            else:
                logger.warning(f"Feature '{feature}' has no variance, excluding")
                missing_features.append(f"{feature} (no variance)")
        else:
            missing_features.append(feature)

    if missing_features:
        logger.warning(f"Missing or invalid features: {missing_features}")

    if len(available_features) < 2:
        raise DataValidationError(
            f"Insufficient valid features: {available_features}. "
            f"Need at least 2 features for anomaly detection."
        )

    logger.info(f"Using {len(available_features)} valid features: {available_features}")
    return available_features


def load_data(filepath: str, config: Dict) -> Tuple[np.ndarray, Union[StandardScaler, RobustScaler], List[str]]:
    """Load and preprocess system metrics with enhanced validation"""
    log_memory_usage("start")

    try:
        df = pd.read_csv(filepath)
        logger.info(f"Loaded {len(df)} rows, {len(df.columns)} columns from {filepath}")
        log_memory_usage("after_load")
    except FileNotFoundError:
        raise FileNotFoundError(f"Input file not found: {filepath}")
    except pd.errors.EmptyDataError:
        raise DataValidationError("Input CSV file is empty")
    except Exception as e:
        raise DataValidationError(f"Failed to load CSV: {str(e)}")

    # Validate data
    validate_data(df, config)

    # Drop unused columns
    columns_to_drop = [col for col in config["drop_columns"] if col in df.columns]
    if columns_to_drop:
        df = df.drop(columns=columns_to_drop)
        logger.info(f"Dropped {len(columns_to_drop)} unused columns")

    # Calculate memory usage percentage with better error handling
    if config["memory_usage_ratio"]:
        if "memory_used_kb" in df.columns and "memory_total_kb" in df.columns:
            # Handle division by zero and negative values
            valid_mask = (df["memory_total_kb"] > 0) & (df["memory_used_kb"] >= 0)
            df["memory_usage"] = 0.0
            df.loc[valid_mask, "memory_usage"] = (
                                                         df.loc[valid_mask, "memory_used_kb"] / df.loc[
                                                     valid_mask, "memory_total_kb"]
                                                 ) * 100

            # Cap at 100% and warn about invalid values
            invalid_count = (~valid_mask).sum()
            if invalid_count > 0:
                logger.warning(f"Found {invalid_count} invalid memory readings, set to 0%")

            df["memory_usage"] = df["memory_usage"].clip(0, 100)
            df = df.drop(columns=["memory_used_kb", "memory_total_kb"])
            logger.info("Calculated memory usage percentage")

    # Validate and select features
    valid_features = validate_features(df, config)

    # Save preprocessed data for debugging
    debug_path = Path(config["output_dir"]) / "preprocessed_data.csv"
    debug_path.parent.mkdir(parents=True, exist_ok=True)
    df[valid_features].to_csv(debug_path, index=False)
    logger.info(f"Saved preprocessed data to {debug_path}")

    # Extract feature matrix
    X = df[valid_features].values

    # Enhanced missing value handling
    if np.isnan(X).any():
        nan_counts = np.isnan(X).sum(axis=0)
        nan_features = [f for i, f in enumerate(valid_features) if nan_counts[i] > 0]
        logger.warning(f"Found NaN values in features: {dict(zip(nan_features, nan_counts[nan_counts > 0]))}")

        # Fill with median for each column
        X_df = pd.DataFrame(X, columns=valid_features)
        X = X_df.fillna(X_df.median()).values
        logger.info("Filled NaN values with column medians")

    # Check for infinite values
    if not np.isfinite(X).all():
        logger.warning("Found infinite values, clipping to finite range")
        X = np.clip(X, -1e10, 1e10)

    log_memory_usage("after_preprocessing")
    return X, valid_features


def preprocess_data(X: np.ndarray, feature_names: List[str],
                    robust: bool = True) -> Tuple[np.ndarray, Union[StandardScaler, RobustScaler]]:
    """Enhanced preprocessing with robust scaling option"""
    if robust:
        scaler = RobustScaler()
        logger.info("Using RobustScaler (less sensitive to outliers)")
    else:
        scaler = StandardScaler()
        logger.info("Using StandardScaler")

    X_scaled = scaler.fit_transform(X)

    # Log scaling statistics
    if hasattr(scaler, 'center_'):  # RobustScaler
        logger.info(f"Robust scaling - Median: {scaler.center_[:3]}..., Scale: {scaler.scale_[:3]}...")
    else:  # StandardScaler
        logger.info(f"Standard scaling - Mean: {scaler.mean_[:3]}..., Scale: {scaler.scale_[:3]}...")

    # Validate scaled data
    if not np.isfinite(X_scaled).all():
        raise DataValidationError("Scaling produced non-finite values")

    return X_scaled, scaler


def enhanced_evaluation(model: IsolationForest, X: np.ndarray,
                        contamination: float) -> Dict:
    """Enhanced model evaluation with percentiles and better metrics"""
    predictions = model.predict(X)
    scores = model.decision_function(X)

    n_anomalies = (predictions == -1).sum()
    anomaly_ratio = n_anomalies / len(predictions)

    # Calculate score percentiles
    percentiles = [1, 5, 10, 25, 50, 75, 90, 95, 99]
    score_percentiles = {
        f"p{p}": float(np.percentile(scores, p)) for p in percentiles
    }

    # Threshold analysis
    threshold = model.offset_
    scores_below_threshold = (scores < threshold).sum()

    metrics = {
        "n_samples": len(X),
        "n_anomalies_detected": int(n_anomalies),
        "anomaly_ratio": float(anomaly_ratio),
        "expected_contamination": contamination,
        "contamination_error": float(abs(anomaly_ratio - contamination)),
        "score_threshold": float(threshold),
        "scores_below_threshold": int(scores_below_threshold),
        "mean_anomaly_score": float(scores.mean()),
        "std_anomaly_score": float(scores.std()),
        "min_anomaly_score": float(scores.min()),
        "max_anomaly_score": float(scores.max()),
        "score_range": f"{scores.min():.4f} to {scores.max():.4f}",
        "score_percentiles": score_percentiles,
        "model_health": {
            "good_variance": scores.std() > 0.01,
            "reasonable_contamination": abs(anomaly_ratio - contamination) < 0.05,
            "negative_scores_present": scores.min() < -0.01
        }
    }

    return metrics


def train_model(X: np.ndarray, config: Dict) -> IsolationForest:
    """Enhanced model training with validation"""
    log_memory_usage("before_training")

    # Validate and fix contamination parameter
    if config['contamination'] <= 0 or config['contamination'] >= 0.5:
        logger.warning(f"Contamination {config['contamination']} is unrealistic. Using 0.10")
        config['contamination'] = 0.10  # Fix: Update config directly

    contamination = config['contamination']

    # Enhanced model configuration
    model_params = {
        'n_estimators': config['n_estimators'],
        'contamination': contamination,
        'random_state': config['random_state'],
        'max_samples': config.get('max_samples', 'auto'),
        'bootstrap': config.get('bootstrap', False),
        'verbose': 1,
        'n_jobs': -1
    }

    logger.info(f"Training Isolation Forest with parameters: {model_params}")

    model = IsolationForest(**model_params)

    # Split data for validation if specified
    if config.get("validation_split", 0) > 0:
        X_train, X_val = train_test_split(
            X,
            test_size=config["validation_split"],
            random_state=config["random_state"],
            shuffle=True
        )
        logger.info(f"Training on {len(X_train)} samples, validating on {len(X_val)}")

        # Train model
        model.fit(X_train)
        log_memory_usage("after_training")

        # Evaluate on validation set
        val_metrics = enhanced_evaluation(model, X_val, contamination)
        logger.info("Validation Results:")
        logger.info(f"  Anomaly ratio: {val_metrics['anomaly_ratio']:.3f} (expected: {contamination:.3f})")
        logger.info(f"  Score range: {val_metrics['score_range']}")
        logger.info(f"  Model health: {val_metrics['model_health']}")

        # Health checks
        health = val_metrics['model_health']
        if not health['good_variance']:
            logger.warning("Low score variance - model may not discriminate well")
        if not health['reasonable_contamination']:
            logger.warning(
                f"Large contamination mismatch: expected {contamination:.3f}, got {val_metrics['anomaly_ratio']:.3f}")
        if not health['negative_scores_present']:
            logger.warning("No strong negative scores - model may not be detecting anomalies properly")

    else:
        logger.info(f"Training on full dataset: {len(X)} samples")
        model.fit(X)
        log_memory_usage("after_training")

    return model


def export_model_with_validation(model: IsolationForest, scaler: Union[StandardScaler, RobustScaler],
                                 feature_names: List[str], output_dir: str,
                                 config: Dict, metrics: Dict) -> None:
    """Enhanced model export with validation and error handling"""
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Create feature order hash for consistency validation
    feature_hash = hashlib.md5(json.dumps(feature_names, sort_keys=True).encode()).hexdigest()

    # Enhanced metadata
    metadata = {
        "model_info": {
            "type": "IsolationForest",
            "version": "1.1.0",
            "training_date": datetime.now().isoformat(),
            "sklearn_version": None,  # Will be filled if available
            "feature_order_hash": feature_hash
        },
        "features": {
            "names": feature_names,
            "count": len(feature_names),
            "order_hash": feature_hash
        },
        "preprocessing": {
            "scaler_type": type(scaler).__name__,
            "robust_scaling": isinstance(scaler, RobustScaler),
            "scaling_params": {
                "center_" if hasattr(scaler, 'center_') else "mean_":
                    (scaler.center_ if hasattr(scaler, 'center_') else scaler.mean_).tolist(),
                "scale_": scaler.scale_.tolist()
            }
        },
        "config": config,
        "metrics": metrics,
        "model_parameters": {
            "contamination": model.contamination,
            "n_estimators": model.n_estimators,
            "max_samples": model.max_samples,
            "offset_": float(model.offset_)
        }
    }

    # Add sklearn version if available
    try:
        import sklearn
        metadata["model_info"]["sklearn_version"] = sklearn.__version__
    except ImportError:
        pass

    # Save metadata
    with open(output_path / "metadata.json", "w") as f:
        json.dump(metadata, f, indent=2)

    # Export ONNX model with enhanced error handling
    onnx_success = False
    try:
        import onnx
        from skl2onnx import convert_sklearn
        from skl2onnx.common.data_types import FloatTensorType
        import sklearn

        logger.info(f"Exporting ONNX model (sklearn {sklearn.__version__})")

        initial_type = [('float_input', FloatTensorType([None, len(feature_names)]))]

        # Handle different sklearn versions
        convert_params = {
            'model': model,
            'initial_types': initial_type,
            'target_opset': {
                '': config["onnx_opset"],
                'ai.onnx.ml': config["ai_onnx_ml_opset"]
            }
        }

        onx = convert_sklearn(**convert_params)

        # Validate ONNX model
        onnx.checker.check_model(onx)

        with open(output_path / "model.onnx", "wb") as f:
            f.write(onx.SerializeToString())

        onnx_success = True
        logger.info("ONNX model exported and validated successfully")

    except ImportError as e:
        logger.warning(f"ONNX dependencies not available: {e}")
    except Exception as e:
        logger.error(f"Failed to export ONNX model: {e}")
        logger.info("Continuing with other export formats...")

    # Export scaler parameters (enhanced for Rust integration)
    scaler_params = {
        "scaler_type": type(scaler).__name__,
        "parameters": {
            "center_" if hasattr(scaler, 'center_') else "mean_":
                (scaler.center_ if hasattr(scaler, 'center_') else scaler.mean_).tolist(),
            "scale_": scaler.scale_.tolist()
        },
        "feature_names": feature_names,
        "n_features": len(feature_names),
        "feature_order_hash": feature_hash,
        "rust_integration": {
            "preprocessing_formula": "robust_scaling" if isinstance(scaler, RobustScaler) else "standard_scaling",
            "expected_input_shape": [-1, len(feature_names)]
        }
    }

    with open(output_path / "scaler.json", "w") as f:
        json.dump(scaler_params, f, indent=2)

    # Export complete model bundle with joblib
    model_bundle = {
        "model": model,
        "scaler": scaler,
        "features": feature_names,
        "metadata": metadata,
        "feature_hash": feature_hash
    }

    joblib.dump(model_bundle, output_path / "model.joblib")

    # Create integration guide
    integration_guide = {
        "rust_integration": {
            "onnx_model": "model.onnx" if onnx_success else None,
            "scaler_config": "scaler.json",
            "feature_order": feature_names,
            "expected_contamination": config["contamination"],
            "anomaly_threshold": float(model.offset_),
            "preprocessing_steps": [
                f"1. Apply {type(scaler).__name__} using parameters from scaler.json",
                "2. Feed scaled features to ONNX model",
                "3. Compare output score to threshold for anomaly detection"
            ],
            "rust_example": {
                "threshold_check": "anomaly = score < threshold",
                "confidence": "confidence = abs(score - threshold)"
            }
        }
    }

    with open(output_path / "integration_guide.json", "w") as f:
        json.dump(integration_guide, f, indent=2)

    logger.info(f"Model successfully exported to {output_dir}")
    logger.info(f"Files created: metadata.json, scaler.json, model.joblib, integration_guide.json" +
                (", model.onnx" if onnx_success else ""))


def parse_arguments():
    """Enhanced argument parsing"""
    parser = argparse.ArgumentParser(
        description="Enhanced Isolation Forest trainer for system metrics anomaly detection",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s --input data.csv --contamination 0.05
  %(prog)s --config custom_config.json --output ./models
  %(prog)s --input data.csv --no-robust-scaling --validation-split 0.3
        """
    )

    parser.add_argument(
        "--input", "-i",
        default=DEFAULT_CONFIG["input_csv"],
        help="Input CSV file path (default: %(default)s)"
    )
    parser.add_argument(
        "--output", "-o",
        default=DEFAULT_CONFIG["output_dir"],
        help="Output directory for model assets (default: %(default)s)"
    )
    parser.add_argument(
        "--contamination", "-c",
        type=float,
        default=DEFAULT_CONFIG["contamination"],
        help="Expected proportion of anomalies (0.01-0.49, default: %(default)s)"
    )
    parser.add_argument(
        "--config", "-f",
        help="Path to JSON config file (overrides defaults)"
    )
    parser.add_argument(
        "--no-robust-scaling",
        action="store_true",
        help="Use StandardScaler instead of RobustScaler"
    )
    parser.add_argument(
        "--validation-split",
        type=float,
        help="Fraction of data to use for validation (default: from config)"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose logging"
    )

    return parser.parse_args()


def load_config(args) -> Dict:
    """Enhanced configuration loading"""
    config = DEFAULT_CONFIG.copy()

    # Load from config file if provided
    if args.config:
        try:
            with open(args.config, 'r') as f:
                file_config = json.load(f)
                config.update(file_config)
                logger.info(f"Loaded configuration from {args.config}")
        except Exception as e:
            logger.error(f"Failed to load config file: {e}")
            raise

    # Override with command line arguments
    config["input_csv"] = args.input
    config["output_dir"] = args.output
    config["contamination"] = args.contamination

    if args.no_robust_scaling:
        config["robust_scaling"] = False

    if args.validation_split is not None:
        config["validation_split"] = args.validation_split

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    # Validate contamination
    if config["contamination"] <= 0 or config["contamination"] >= 0.5:
        logger.warning(f"Contamination {config['contamination']} is unrealistic")
        config["contamination"] = 0.10

    return config


def main():
    """Enhanced main pipeline with comprehensive error handling"""
    args = parse_arguments()

    try:
        # Load configuration
        config = load_config(args)

        logger.info("=" * 60)
        logger.info("ENHANCED ISOLATION FOREST TRAINING")
        logger.info("=" * 60)
        logger.info(f"Configuration:")
        for key, value in config.items():
            if key not in ['drop_columns']:  # Skip verbose lists
                logger.info(f"  {key}: {value}")

        log_memory_usage("initialization")

        # Load and preprocess data
        logger.info("\n📊 Loading and preprocessing data...")
        X, feature_names = load_data(config['input_csv'], config)
        logger.info(f"Final dataset: {X.shape[0]} samples × {X.shape[1]} features")
        logger.info(f"Features: {feature_names}")

        # Preprocess data with scaling
        logger.info("\n🔧 Applying feature scaling...")
        X_scaled, scaler = preprocess_data(X, feature_names, config['robust_scaling'])

        # Train model
        logger.info("\n🤖 Training Isolation Forest model...")
        model = train_model(X_scaled, config)

        # Evaluate model
        logger.info("\n📈 Evaluating model performance...")
        metrics = enhanced_evaluation(model, X_scaled, config['contamination'])

        # Display results
        logger.info(f"\n📊 TRAINING RESULTS:")
        logger.info(f"  Dataset: {metrics['n_samples']} samples")
        logger.info(f"  Detected anomalies: {metrics['n_anomalies_detected']} ({metrics['anomaly_ratio']:.2%})")
        logger.info(f"  Expected contamination: {metrics['expected_contamination']:.2%}")
        logger.info(f"  Contamination error: {metrics['contamination_error']:.3f}")
        logger.info(f"  Score range: {metrics['score_range']}")
        logger.info(f"  Anomaly threshold: {metrics['score_threshold']:.4f}")
        logger.info(f"  Model health: {metrics['model_health']}")

        # Export model
        logger.info("\n💾 Exporting trained model...")
        export_model_with_validation(model, scaler, feature_names,
                                     config['output_dir'], config, metrics)

        log_memory_usage("completion")

        # Final summary
        logger.info("\n" + "=" * 60)
        logger.info("✅ TRAINING COMPLETED SUCCESSFULLY!")
        logger.info("=" * 60)
        logger.info(f"Model exported to: {config['output_dir']}")
        logger.info(f"Features monitored: {len(feature_names)} ({', '.join(feature_names)})")
        logger.info(f"Anomaly detection threshold: {model.offset_:.4f}")
        logger.info(f"Expected anomaly rate: {config['contamination']:.1%}")

        health_issues = [k for k, v in metrics['model_health'].items() if not v]
        if health_issues:
            logger.warning(f"⚠️  Health concerns: {health_issues}")
        else:
            logger.info("✅ All model health checks passed!")

        return 0

    except Exception as e:
        logger.error(f"\n❌ Training failed: {str(e)}")
        logger.exception("Full traceback:")
        return 1


if __name__ == "__main__":
    exit(main())