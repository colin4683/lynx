import pandas as pd
from prophet import Prophet
import matplotlib.pyplot as plt

# Load metrics with timestamp
df = pd.read_csv("augmented_metrics2.csv", parse_dates=["time"])

# Remove timezone from time column if present
df["time"] = df["time"].dt.tz_localize(None)

# Forecast CPU usage
cpu_df = df[["time", "cpu_usage"]].rename(columns={"time": "ds", "cpu_usage": "y"})
# Plot original data
plt.figure(figsize=(10, 6))
plt.plot(cpu_df["ds"], cpu_df["y"], label="CPU Usage")
plt.xlabel("Time")
plt.ylabel("CPU Usage (%)")
plt.title("CPU Usage Over Time")
plt.legend()
plt.savefig("cpu_usage_over_time.png")


# Train model
model = Prophet()
model.fit(cpu_df)

# Predict next 24 hours (adjust 'periods' for longer forecasts)
future = model.make_future_dataframe(periods=60, freq="min")
forecast = model.predict(future)

# Show last 200 points + forecast
fig = model.plot(forecast)
fig.savefig("cpu_forecast.png")
fig2 = model.plot_components(forecast)
fig2.savefig("cpu_forecast_components.png")