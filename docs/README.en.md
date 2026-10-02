# Pinmeter

<img src="../src/backend/host/icons/128x128.png" alt="Pinmeter icon" width="96" height="96" />

[简体中文](../README.md) | **English**

A lightweight system monitor for viewing your computer's status in the main window, system tray, and Windows taskbar.

The [v0.1.1 Windows x64 preview](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.1) is available. macOS and Linux have not completed testing on real devices.

## Features

- **System monitoring**: CPU, memory, GPU, network speeds, and supported temperature readings, with five minutes of recent trends.
- **Taskbar display**: Compact readings across two rows, with system tray support.
- **Hardware information**: View your processor, graphics cards, memory, disks, monitors, network adapters, and other devices.
- **Per-app network control**: Track traffic by application, set upload and download limits, and block or restore network access.
- **IP inspection**: Check your public IP, IP details, connectivity, and third-party service status.
- **Preferences**: Light and dark themes, sampling interval, network adapter selection, and launch at startup. Changes are saved immediately.

Network control and proxy / VPN scenarios are still being improved. See [development progress](development/README.md) (in Chinese) for tested scenarios and known limitations.

## Screenshots

Captured from the v0.1.0 development build on Windows. The screenshots show the Chinese interface. Click an image to view it at full size.

### Overview

[![Pinmeter overview: CPU, GPU, memory, network speeds, and resource trends, with Hardware Information in the sidebar](development/v0.1.0-main-window/assets/readme-overview-native-dark.png)](development/v0.1.0-main-window/assets/readme-overview-native-dark.png)

### Taskbar

[![Pinmeter taskbar readings: upload and download speeds, CPU and GPU usage, and temperatures](development/v0.1.0-taskbar-display/assets/readme-taskbar-readouts.png)](development/v0.1.0-taskbar-display/assets/readme-taskbar-readouts.png)

## Usage

Download the [Windows installer](https://github.com/tsuixl/Pinmeter/releases/download/v0.1.1/Pinmeter_0.1.1_x64-setup.exe) or [Windows portable ZIP](https://github.com/tsuixl/Pinmeter/releases/download/v0.1.1/Pinmeter-0.1.1-preview-windows-x64.zip). See the [release notes](https://github.com/tsuixl/Pinmeter/releases/tag/v0.1.1) for source code, checksums, and known limitations.

On Windows, Pinmeter requests administrator privileges at startup. Features such as CPU temperature monitoring depend on hardware and driver support; the app explains when a required driver is missing.

If PawnIO is required, click “Download and install driver” on the CPU page. Pinmeter downloads it from the [official source](https://pawnio.eu/), verifies it, installs it, and checks again automatically. Internet access and administrator privileges are required; the installer is not bundled with Pinmeter.

Minimizing hides the window in the system tray. Click the tray icon to restore it. Closing the window lets you choose between minimizing and exiting. The Network page and Runtime Status panel provide an option to remove all network restrictions.

Keep the entire application directory when using a portable build, and fully exit the old version before switching. You can also build from source by running these commands from the repository root:

```powershell
npm.cmd --prefix src/frontend ci
powershell -ExecutionPolicy Bypass -File tools/dev.ps1 build
```

You need Node.js 24, Rust with the MSVC toolchain, Visual Studio C++ tools, and WebView2. See the [environment prerequisites](https://v2.tauri.app/start/prerequisites/). The build prints the actual path to `src/backend/target/testN/Pinmeter.exe`.

See the [development guide](development/README.md#本地运行与构建) for development mode, checks, and configuration paths, or the [documentation index](README.md) for design and implementation details. These documents are in Chinese.

## License and acknowledgments

Pinmeter is licensed under [AGPL-3.0-only](../LICENSE). Copyright (C) 2026 Pinmeter contributors. Third-party components retain their own terms; see the [third-party notices](../src/backend/host/resources/legal/THIRD-PARTY-NOTICES.txt) and [source code information](../src/backend/host/resources/legal/SOURCE_CODE.txt).

Thanks to [Sakani](https://github.com/samzydd/Sakani-design-system) for its visual design and components, [one-ip](https://github.com/zhihui-hu/one-ip) for its IP inspection implementation, and [Cindy](https://github.com/makecindy/cindy) for window integration ideas. Hardware and network features use LibreHardwareMonitor, PawnIO, and WinDivert. The interface is built with React, Tauri, Lucide, and Geist.

The Windows implementation draws on TrafficMonitor, System Informer, and windows_exporter; network control draws on Throttle and BandwidthDesk. See the [Windows monitoring research](research/windows-monitoring.md) (in Chinese) and [network control notices](../src/backend/platform/network-control/THIRD-PARTY-NOTICES.txt) for sources and scope.
