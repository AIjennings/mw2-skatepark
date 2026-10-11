@echo off
title MW2 + Skate 3 Generative Skatepark
echo ======================================================================
echo Launching MW2 + Skate 3 Continuous Generative Skatepark...
echo Procedural Horizon Streamer Active!
echo Press 'J' in game (or click both controller thumbsticks) to skate!
echo ======================================================================
cd /d "%~dp0"
if exist "%~dp0target\play\iw4l.exe" (
    start "MW2 Skatepark" /d "%~dp0" "%~dp0target\play\iw4l.exe" map mp_rust --cmds "spawn 0; skate on"
) else (
    echo [ERROR] target\play\iw4l.exe not found! Run 'cargo build --profile play -p launcher' first.
    pause
)
