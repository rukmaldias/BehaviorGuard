"""
BehaviorGuard — Phase 2 autoencoder training.

Trains a shallow autoencoder on enrolled feature vectors.
The trained model replaces the z-score scorer in Phase 2.

Usage:
    python train_autoencoder.py --data enrolled_features.json --out model.tflite

Input JSON format (produced by exporting feature vectors from the SDK):
    [
        [f0, f1, ..., f31],   # session 1
        [f0, f1, ..., f31],   # session 2
        ...
    ]
"""

import argparse
import json
import numpy as np

FEATURE_DIM = 32


def load_data(path: str) -> np.ndarray:
    with open(path) as f:
        vectors = json.load(f)
    data = np.array(vectors, dtype=np.float32)
    assert data.shape[1] == FEATURE_DIM, f"Expected {FEATURE_DIM} features, got {data.shape[1]}"
    return data


def normalise(data: np.ndarray):
    mean = data.mean(axis=0)
    std = data.std(axis=0) + 1e-6
    return (data - mean) / std, mean, std


def build_autoencoder(input_dim: int, latent_dim: int = 8):
    try:
        import tensorflow as tf
    except ImportError:
        raise SystemExit("Install TensorFlow: pip install tensorflow")

    inputs = tf.keras.Input(shape=(input_dim,))
    encoded = tf.keras.layers.Dense(16, activation="relu")(inputs)
    latent = tf.keras.layers.Dense(latent_dim, activation="relu")(encoded)
    decoded = tf.keras.layers.Dense(16, activation="relu")(latent)
    outputs = tf.keras.layers.Dense(input_dim, activation="linear")(decoded)

    model = tf.keras.Model(inputs, outputs)
    model.compile(optimizer="adam", loss="mse")
    return model


def export_tflite(model, out_path: str):
    import tensorflow as tf
    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    tflite_model = converter.convert()
    with open(out_path, "wb") as f:
        f.write(tflite_model)
    print(f"Exported: {out_path} ({len(tflite_model):,} bytes)")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", required=True, help="Path to enrolled_features.json")
    parser.add_argument("--out", default="model.tflite", help="Output .tflite path")
    parser.add_argument("--epochs", type=int, default=100)
    parser.add_argument("--latent", type=int, default=8)
    args = parser.parse_args()

    data = load_data(args.data)
    print(f"Loaded {len(data)} sessions, {FEATURE_DIM} features each")

    normalised, mean, std = normalise(data)
    np.save("normalise_mean.npy", mean)
    np.save("normalise_std.npy", std)
    print(f"Saved normalisation parameters (mean, std)")

    model = build_autoencoder(FEATURE_DIM, args.latent)
    model.summary()

    model.fit(
        normalised, normalised,
        epochs=args.epochs,
        batch_size=min(16, len(data)),
        validation_split=0.2 if len(data) >= 10 else 0.0,
        verbose=1,
    )

    # Compute reconstruction error threshold on training data
    preds = model.predict(normalised)
    errors = np.mean((normalised - preds) ** 2, axis=1)
    threshold = float(np.percentile(errors, 95))
    print(f"Suggested anomaly threshold (95th percentile): {threshold:.4f}")

    export_tflite(model, args.out)


if __name__ == "__main__":
    main()
