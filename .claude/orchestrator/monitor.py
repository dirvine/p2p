# \!/usr/bin/env python3
import json
import os
import subprocess
import time
from datetime import datetime, timezone


def monitor_progress():
    project_root = "/Users/davidirvine/Desktop/Devel/projects/p2p"
    state_file = f"{project_root}/.claude/orchestrator/state.json"
    signals_dir = f"{project_root}/.claude/orchestrator/signals"

    print("🎯 PRODUCTION ORCHESTRATOR - ACTIVE MONITORING")
    print("=" * 60)

    while True:
        try:
            # Load current state
            with open(state_file, "r") as f:
                state = json.load(f)

            current_task = state["current_task"]["number"]
            completed = len(state["completed_tasks"])
            total = state["total_tasks"]

            # Check for completion signals
            completed_signals = []
            if os.path.exists(signals_dir):
                for file in os.listdir(signals_dir):
                    if file.startswith("task_") and file.endswith("_complete"):
                        task_num = int(file.split("_")[1])
                        completed_signals.append(task_num)

            # Display status
            progress_bar = "█" * (completed * 40 // total) + "░" * (
                40 - (completed * 40 // total)
            )
            print(
                f"\n⏰ {datetime.now().strftime('%H:%M:%S')} - MONITORING TASK {current_task}"
            )
            print(
                f"📊 Progress: [{progress_bar}] {completed}/{total} ({completed*100//total}%)"
            )

            if current_task in completed_signals:
                print(
                    f"✅ TASK {current_task} COMPLETED\! Progressing to task {current_task + 1}..."
                )

                # Update state for next task
                if current_task < total:
                    next_task = current_task + 1
                    completed_task = {
                        "number": current_task,
                        "completed": datetime.now(timezone.utc).isoformat(),
                        "iterations": 1,
                        "notes": f"Task {current_task} completed successfully",
                    }
                    state["completed_tasks"].append(completed_task)
                    state["current_task"] = {
                        "number": next_task,
                        "file": f"task-{next_task:03d}-*.md",
                        "started": datetime.now(timezone.utc).isoformat(),
                        "iterations": 0,
                        "status": "starting",
                    }

                    with open(state_file, "w") as f:
                        json.dump(state, f, indent=2)

                    print(f"🚀 STARTING TASK {next_task}")
                else:
                    print("🎉 ALL TASKS COMPLETED\!")
                    break
            else:
                print(f"⏳ Task {current_task} in progress...")

            print("=" * 60)
            time.sleep(30)  # Check every 30 seconds

        except Exception as e:
            print(f"❌ Monitoring error: {e}")
            time.sleep(60)

    print("🏁 Production readiness sprint completed\!")


if __name__ == "__main__":
    monitor_progress()
