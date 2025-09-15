#!/bin/bash
# Focused & Enhanced Stress Scheduler for CPU, Load, and Network

DURATION=172800  # 48 hours in seconds
LOG_FILE="/var/log/stress_metrics.csv"
STRESSORS=(cpu cpu-load udp vm cache sock matrix)
MAX_LEVEL=10

echo "🔵 [$(date '+%Y-%m-%d %H:%M:%S')] Starting 48-hour focused stress test session"
echo "🔵 [$(date '+%Y-%m-%d %H:%M:%S')] Output will be saved to: $LOG_FILE"

# Write CSV header
echo "timestamp,stressor,level,cpu_load,mem_used,load_1" > "$LOG_FILE"
end_time=$(( $(date +%s) + DURATION ))

while [ $(date +%s) -lt $end_time ]; do
    stressor=$(shuf -n1 -e "${STRESSORS[@]}")
    stress_duration=$(( 120 + RANDOM % 300 ))  # 2-5 minutes
    normal_duration=$(( 300 + RANDOM % 600 ))    # 5-10 minutes
    level=$(( 2 + RANDOM % (MAX_LEVEL-1) ))    # Level 2-10

    echo "🟢 [$(date '+%Y-%m-%d %H:%M:%S')] Starting stress cycle:"
    echo "   - Stressor: $stressor"
    echo "   - Level: $level"
    echo "   - Duration: ${stress_duration}s"
    echo "   - Normal period after: ${normal_duration}s"

    echo "$(date -u +%FT%TZ),STRESS_START,$stressor,$level" >> "$LOG_FILE"
    echo "   📝 Logged stress start marker"

    echo "   ⚡ Running stressor..."
    stdbuf -oL stress-ng --$stressor $level --timeout ${stress_duration}s --metrics-brief --aggressive 2>&1 | tee -a "$LOG_FILE"

    echo "$(date -u +%FT%TZ),STRESS_END,$stressor,$level" >> "$LOG_FILE"
    echo "   📝 Logged stress end marker"

    echo "🟦 [$(date '+%Y-%m-%d %H:%M:%S')] Entering normal operation period (${normal_duration}s)"
    for ((i=1; i<=$normal_duration; i++)); do
        printf "\r   ⏳ %02d/%02ds elapsed" $i $normal_duration
        sleep 1
    done
    echo -e "\n   ✅ Normal period completed"
done

echo "🔴 [$(date '+%Y-%m-%d %H:%M:%S')] Stress test session completed"
