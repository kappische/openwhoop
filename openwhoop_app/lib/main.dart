import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:permission_handler/permission_handler.dart';
import 'whoop_protocol.dart';
import 'heart_rate_screen.dart';

void main() {
  runApp(const OpenWhoopApp());
}

class OpenWhoopApp extends StatelessWidget {
  const OpenWhoopApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'OpenWhoop',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.red,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      home: const ScanScreen(),
    );
  }
}

class ScanScreen extends StatefulWidget {
  const ScanScreen({super.key});

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  List<ScanResult> _scanResults = [];
  bool _isScanning = false;
  String _status = 'Tap Scan to find your Whoop';

  @override
  void initState() {
    super.initState();
    _requestPermissions();
  }

  Future<void> _requestPermissions() async {
    await [
      Permission.bluetooth,
      Permission.bluetoothScan,
      Permission.bluetoothConnect,
      Permission.location,
    ].request();
  }

  Future<void> _startScan() async {
    setState(() {
      _scanResults = [];
      _isScanning = true;
      _status = 'Scanning for Whoop devices...';
    });

    // Check if Bluetooth is on
    if (await FlutterBluePlus.adapterState.first != BluetoothAdapterState.on) {
      setState(() {
        _status = 'Please turn on Bluetooth';
        _isScanning = false;
      });
      return;
    }

    // Listen for scan results
    FlutterBluePlus.scanResults.listen((results) {
      setState(() {
        // Filter for Whoop devices (they advertise with the Whoop service UUID)
        _scanResults = results.where((r) {
          final name = r.device.platformName.toLowerCase();
          final serviceUuids = r.advertisementData.serviceUuids;

          // Check if it's a Whoop device
          return name.contains('whoop') ||
              serviceUuids.any((uuid) =>
                uuid.toString().toLowerCase().contains('61080001'));
        }).toList();
      });
    });

    // Start scanning
    await FlutterBluePlus.startScan(
      timeout: const Duration(seconds: 15),
      withServices: [Guid('61080001-8d6d-82b8-614a-1c8cb0f8dcc6')],
    );

    setState(() {
      _isScanning = false;
      _status = _scanResults.isEmpty
          ? 'No Whoop devices found. Make sure your Whoop is nearby and not connected to another device.'
          : 'Found ${_scanResults.length} device(s). Tap to connect.';
    });
  }

  Future<void> _connectToDevice(BluetoothDevice device) async {
    setState(() {
      _status = 'Connecting to ${device.platformName}...';
    });

    try {
      await device.connect(timeout: const Duration(seconds: 10));

      if (!mounted) return;

      Navigator.push(
        context,
        MaterialPageRoute(
          builder: (context) => HeartRateScreen(device: device),
        ),
      );
    } catch (e) {
      setState(() {
        _status = 'Connection failed: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('OpenWhoop'),
        centerTitle: true,
      ),
      body: Column(
        children: [
          // Status card
          Container(
            width: double.infinity,
            margin: const EdgeInsets.all(16),
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceVariant,
              borderRadius: BorderRadius.circular(16),
            ),
            child: Column(
              children: [
                Icon(
                  _isScanning ? Icons.bluetooth_searching : Icons.watch,
                  size: 48,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 12),
                Text(
                  _status,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyLarge,
                ),
              ],
            ),
          ),

          // Scan button
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: SizedBox(
              width: double.infinity,
              height: 56,
              child: FilledButton.icon(
                onPressed: _isScanning ? null : _startScan,
                icon: _isScanning
                    ? const SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.bluetooth_searching),
                label: Text(_isScanning ? 'Scanning...' : 'Scan for Whoop'),
              ),
            ),
          ),

          const SizedBox(height: 16),

          // Device list
          Expanded(
            child: ListView.builder(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              itemCount: _scanResults.length,
              itemBuilder: (context, index) {
                final result = _scanResults[index];
                final device = result.device;

                return Card(
                  margin: const EdgeInsets.only(bottom: 8),
                  child: ListTile(
                    leading: const CircleAvatar(
                      child: Icon(Icons.watch),
                    ),
                    title: Text(
                      device.platformName.isNotEmpty
                          ? device.platformName
                          : 'Whoop Device',
                    ),
                    subtitle: Text('Signal: ${result.rssi} dBm'),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => _connectToDevice(device),
                  ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
