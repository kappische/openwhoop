import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:fl_chart/fl_chart.dart';
import 'whoop_protocol.dart';

class HeartRateScreen extends StatefulWidget {
  final BluetoothDevice device;

  const HeartRateScreen({super.key, required this.device});

  @override
  State<HeartRateScreen> createState() => _HeartRateScreenState();
}

class _HeartRateScreenState extends State<HeartRateScreen>
    with SingleTickerProviderStateMixin {
  late WhoopDevice _whoopDevice;
  String _status = 'Connecting...';
  int _currentBpm = 0;
  final List<FlSpot> _heartRateHistory = [];
  int _dataPointIndex = 0;
  bool _isConnected = false;
  late AnimationController _pulseController;
  late Animation<double> _pulseAnimation;

  // Stats
  int _minBpm = 999;
  int _maxBpm = 0;
  int _avgBpm = 0;
  int _readingCount = 0;
  int _totalBpm = 0;

  @override
  void initState() {
    super.initState();
    _whoopDevice = WhoopDevice(widget.device);
    _setupPulseAnimation();
    _initializeDevice();
  }

  void _setupPulseAnimation() {
    _pulseController = AnimationController(
      duration: const Duration(milliseconds: 800),
      vsync: this,
    );
    _pulseAnimation = Tween<double>(begin: 1.0, end: 1.15).animate(
      CurvedAnimation(parent: _pulseController, curve: Curves.easeInOut),
    );
  }

  Future<void> _initializeDevice() async {
    _whoopDevice.onHeartRateReceived = _onHeartRateReceived;
    _whoopDevice.onStatusChanged = (status) {
      if (mounted) {
        setState(() => _status = status);
      }
    };

    try {
      await _whoopDevice.initialize();
      if (mounted) {
        setState(() => _isConnected = true);
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _status = 'Error: $e';
          _isConnected = false;
        });
      }
    }
  }

  void _onHeartRateReceived(HeartRateReading reading) {
    if (!mounted) return;

    setState(() {
      _currentBpm = reading.bpm;

      // Update stats
      _readingCount++;
      _totalBpm += reading.bpm;
      _avgBpm = _totalBpm ~/ _readingCount;
      if (reading.bpm < _minBpm) _minBpm = reading.bpm;
      if (reading.bpm > _maxBpm) _maxBpm = reading.bpm;

      // Add to chart history (keep last 60 readings)
      _heartRateHistory.add(FlSpot(_dataPointIndex.toDouble(), reading.bpm.toDouble()));
      _dataPointIndex++;
      if (_heartRateHistory.length > 60) {
        _heartRateHistory.removeAt(0);
      }
    });

    // Pulse animation
    _pulseController.forward().then((_) => _pulseController.reverse());
  }

  Color _getBpmColor(int bpm) {
    if (bpm < 60) return Colors.blue;
    if (bpm < 100) return Colors.green;
    if (bpm < 140) return Colors.orange;
    return Colors.red;
  }

  String _getBpmZone(int bpm) {
    if (bpm < 60) return 'Resting';
    if (bpm < 100) return 'Normal';
    if (bpm < 140) return 'Elevated';
    if (bpm < 170) return 'High';
    return 'Max';
  }

  @override
  void dispose() {
    _pulseController.dispose();
    _whoopDevice.disconnect();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final bpmColor = _getBpmColor(_currentBpm);

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.device.platformName.isNotEmpty
            ? widget.device.platformName
            : 'Whoop'),
        actions: [
          // Connection status indicator
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Icon(
              _isConnected ? Icons.bluetooth_connected : Icons.bluetooth_disabled,
              color: _isConnected ? Colors.green : Colors.red,
            ),
          ),
        ],
      ),
      body: SingleChildScrollView(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              // Status
              Text(
                _status,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 24),

              // Main heart rate display
              AnimatedBuilder(
                animation: _pulseAnimation,
                builder: (context, child) {
                  return Transform.scale(
                    scale: _currentBpm > 0 ? _pulseAnimation.value : 1.0,
                    child: Container(
                      width: 200,
                      height: 200,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: bpmColor.withOpacity(0.2),
                        border: Border.all(color: bpmColor, width: 4),
                        boxShadow: [
                          BoxShadow(
                            color: bpmColor.withOpacity(0.3),
                            blurRadius: 20,
                            spreadRadius: 5,
                          ),
                        ],
                      ),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(Icons.favorite, color: bpmColor, size: 40),
                          const SizedBox(height: 8),
                          Text(
                            _currentBpm > 0 ? '$_currentBpm' : '--',
                            style: Theme.of(context).textTheme.displayLarge?.copyWith(
                              fontWeight: FontWeight.bold,
                              color: bpmColor,
                            ),
                          ),
                          Text(
                            'BPM',
                            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                              color: bpmColor,
                            ),
                          ),
                          if (_currentBpm > 0)
                            Text(
                              _getBpmZone(_currentBpm),
                              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: bpmColor,
                              ),
                            ),
                        ],
                      ),
                    ),
                  );
                },
              ),

              const SizedBox(height: 32),

              // Stats row
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  _StatCard(
                    label: 'MIN',
                    value: _minBpm < 999 ? '$_minBpm' : '--',
                    icon: Icons.arrow_downward,
                    color: Colors.blue,
                  ),
                  _StatCard(
                    label: 'AVG',
                    value: _avgBpm > 0 ? '$_avgBpm' : '--',
                    icon: Icons.show_chart,
                    color: Colors.green,
                  ),
                  _StatCard(
                    label: 'MAX',
                    value: _maxBpm > 0 ? '$_maxBpm' : '--',
                    icon: Icons.arrow_upward,
                    color: Colors.red,
                  ),
                ],
              ),

              const SizedBox(height: 24),

              // Heart rate chart
              if (_heartRateHistory.isNotEmpty)
                Container(
                  height: 200,
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.surfaceVariant,
                    borderRadius: BorderRadius.circular(16),
                  ),
                  child: LineChart(
                    LineChartData(
                      gridData: FlGridData(
                        show: true,
                        drawVerticalLine: false,
                        horizontalInterval: 20,
                        getDrawingHorizontalLine: (value) => FlLine(
                          color: Theme.of(context).colorScheme.outline.withOpacity(0.3),
                          strokeWidth: 1,
                        ),
                      ),
                      titlesData: FlTitlesData(
                        leftTitles: AxisTitles(
                          sideTitles: SideTitles(
                            showTitles: true,
                            reservedSize: 40,
                            interval: 20,
                            getTitlesWidget: (value, meta) => Text(
                              value.toInt().toString(),
                              style: TextStyle(
                                fontSize: 10,
                                color: Theme.of(context).colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ),
                        ),
                        bottomTitles: const AxisTitles(
                          sideTitles: SideTitles(showTitles: false),
                        ),
                        topTitles: const AxisTitles(
                          sideTitles: SideTitles(showTitles: false),
                        ),
                        rightTitles: const AxisTitles(
                          sideTitles: SideTitles(showTitles: false),
                        ),
                      ),
                      borderData: FlBorderData(show: false),
                      minY: 40,
                      maxY: 200,
                      lineBarsData: [
                        LineChartBarData(
                          spots: _heartRateHistory,
                          isCurved: true,
                          color: bpmColor,
                          barWidth: 3,
                          isStrokeCapRound: true,
                          dotData: const FlDotData(show: false),
                          belowBarData: BarAreaData(
                            show: true,
                            color: bpmColor.withOpacity(0.2),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),

              const SizedBox(height: 16),

              // Reading count
              Text(
                '$_readingCount readings received',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;
  final Color color;

  const _StatCard({
    required this.label,
    required this.value,
    required this.icon,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Column(
        children: [
          Icon(icon, color: color, size: 20),
          const SizedBox(height: 4),
          Text(
            value,
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
              color: color,
            ),
          ),
          Text(
            label,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: color,
            ),
          ),
        ],
      ),
    );
  }
}
