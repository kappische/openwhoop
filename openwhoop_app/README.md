# OpenWhoop Mobile App

A Flutter app that connects to your Whoop 4.0 device via Bluetooth and displays your heart rate - no subscription required!

## Features

- Scan and connect to Whoop devices
- Real-time heart rate display with pulse animation
- Heart rate history chart
- Min/Avg/Max statistics
- Heart rate zone indicators (Resting, Normal, Elevated, High, Max)

## Requirements

To build and run this app on your iPhone, you need:

1. **A Mac computer** (required for iOS development)
2. **Xcode** (free from the Mac App Store)
3. **Flutter SDK** (free)
4. **An Apple ID** (free, for running on your personal device)

## Step-by-Step Setup Guide

### Step 1: Install Xcode

1. Open the **App Store** on your Mac
2. Search for "Xcode"
3. Click **Get** / **Install** (it's free but ~12GB)
4. Wait for it to download and install
5. Open Xcode once and accept the license agreement

### Step 2: Install Flutter

Open **Terminal** (find it in Applications > Utilities) and run these commands:

```bash
# Download Flutter
cd ~
git clone https://github.com/flutter/flutter.git -b stable

# Add Flutter to your PATH
echo 'export PATH="$HOME/flutter/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Verify installation
flutter doctor
```

### Step 3: Set Up iOS Development

In Terminal, run:

```bash
# Install iOS development tools
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
sudo xcodebuild -runFirstLaunch

# Accept Xcode license
sudo xcodebuild -license accept

# Install CocoaPods (iOS dependency manager)
sudo gem install cocoapods
```

### Step 4: Build the App

1. Connect your iPhone to your Mac with a USB cable
2. On your iPhone, tap "Trust" when asked to trust this computer
3. In Terminal, navigate to the app folder and run:

```bash
cd /path/to/openwhoop/openwhoop_app

# Get dependencies
flutter pub get

# Install iOS dependencies
cd ios
pod install
cd ..

# Run the app on your iPhone
flutter run
```

### Step 5: Trust the Developer on iPhone

The first time you run the app, your iPhone will say it's from an "Untrusted Developer":

1. On your iPhone, go to **Settings > General > VPN & Device Management**
2. Tap on your Apple ID under "Developer App"
3. Tap **Trust**
4. Go back and open the OpenWhoop app

## Using the App

1. **Make sure your Whoop is NOT connected to the official Whoop app** (turn off Bluetooth on your other devices or unpair it)
2. Open OpenWhoop
3. Tap **Scan for Whoop**
4. Wait for your Whoop to appear in the list
5. Tap on your Whoop device to connect
6. Watch your heart rate in real-time!

## Troubleshooting

### "No Whoop devices found"
- Make sure your Whoop is charged and on your wrist
- Make sure it's not connected to another phone/device
- Try restarting Bluetooth on your iPhone
- Move closer to your Whoop

### "Connection failed"
- Make sure you're not connected via the official Whoop app
- Try restarting your Whoop (take it off the charger and put it back on)
- Restart the OpenWhoop app

### Build errors
Run these commands to clean and rebuild:
```bash
flutter clean
flutter pub get
cd ios && pod install && cd ..
flutter run
```

## How It Works

The app uses Bluetooth Low Energy (BLE) to communicate with your Whoop using the same protocol the official app uses. It:

1. Scans for devices advertising the Whoop service UUID
2. Connects and discovers BLE characteristics
3. Subscribes to data notifications
4. Parses heart rate packets and displays them

## Privacy

- All data stays on your phone
- No internet connection required
- No data is sent anywhere
- Your Whoop still works normally with the official app

## Credits

Based on the reverse-engineered protocol from the [OpenWhoop](https://github.com/openwhoop/openwhoop) project.
