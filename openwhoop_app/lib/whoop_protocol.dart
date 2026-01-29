import 'dart:typed_data';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

/// Whoop BLE Service and Characteristic UUIDs
class WhoopUuids {
  static final Guid service = Guid('61080001-8d6d-82b8-614a-1c8cb0f8dcc6');
  static final Guid cmdToStrap = Guid('61080002-8d6d-82b8-614a-1c8cb0f8dcc6');
  static final Guid cmdFromStrap = Guid('61080003-8d6d-82b8-614a-1c8cb0f8dcc6');
  static final Guid eventsFromStrap = Guid('61080004-8d6d-82b8-614a-1c8cb0f8dcc6');
  static final Guid dataFromStrap = Guid('61080005-8d6d-82b8-614a-1c8cb0f8dcc6');
  static final Guid memfault = Guid('61080007-8d6d-82b8-614a-1c8cb0f8dcc6');
}

/// Packet types from the Whoop protocol
enum PacketType {
  command(35),
  commandResponse(36),
  realtimeData(40),
  historicalData(47),
  realtimeRawData(43),
  event(48),
  metadata(49),
  consoleLogs(50);

  final int value;
  const PacketType(this.value);

  static PacketType? fromValue(int value) {
    for (var type in PacketType.values) {
      if (type.value == value) return type;
    }
    return null;
  }
}

/// Command numbers for the Whoop protocol
enum CommandNumber {
  linkValid(1),
  getMaxProtocolVersion(2),
  toggleRealtimeHr(3),
  reportVersionInfo(7),
  setClock(10),
  getClock(11),
  toggleGenericHrProfile(14),
  toggleR7DataCollection(16),
  abortHistoricalTransmits(20),
  sendHistoricalData(22),
  historicalDataResult(23),
  getBatteryLevel(26),
  getHelloHarvard(35),
  getAdvertisingNameHarvard(76),
  enterHighFreqSync(96),
  exitHighFreqSync(97);

  final int value;
  const CommandNumber(this.value);

  static CommandNumber? fromValue(int value) {
    for (var cmd in CommandNumber.values) {
      if (cmd.value == value) return cmd;
    }
    return null;
  }
}

/// Represents a heart rate reading from the Whoop
class HeartRateReading {
  final int timestamp;
  final int bpm;
  final List<int> rrIntervals;
  final int activity;

  HeartRateReading({
    required this.timestamp,
    required this.bpm,
    required this.rrIntervals,
    required this.activity,
  });

  @override
  String toString() {
    return 'HeartRateReading(bpm: $bpm, rr: $rrIntervals, time: $timestamp)';
  }
}

/// Whoop packet structure and parsing
class WhoopPacket {
  static const int sof = 0xAA;

  final PacketType packetType;
  final int seq;
  final int cmd;
  final List<int> data;

  WhoopPacket({
    required this.packetType,
    required this.seq,
    required this.cmd,
    required this.data,
  });

  /// Calculate CRC8 checksum
  static int crc8(List<int> data) {
    int crc = 0;
    for (var byte in data) {
      crc ^= byte;
      for (var i = 0; i < 8; i++) {
        if ((crc & 0x80) != 0) {
          crc = ((crc << 1) ^ 0x07) & 0xFF;
        } else {
          crc = (crc << 1) & 0xFF;
        }
      }
    }
    return crc;
  }

  /// Calculate CRC32 checksum
  static int crc32(List<int> data) {
    int crc = 0xFFFFFFFF;
    for (var byte in data) {
      crc ^= byte;
      for (var i = 0; i < 8; i++) {
        if ((crc & 1) != 0) {
          crc = ((crc >> 1) ^ 0xEDB88320) & 0xFFFFFFFF;
        } else {
          crc = (crc >> 1) & 0xFFFFFFFF;
        }
      }
    }
    return (~crc) & 0xFFFFFFFF;
  }

  /// Create a framed packet ready to send
  List<int> toFramedPacket() {
    // Create the payload
    final payload = <int>[packetType.value, seq, cmd, ...data];

    // Calculate length (payload + CRC32)
    final length = payload.length + 4;
    final lengthBytes = [length & 0xFF, (length >> 8) & 0xFF];

    // Calculate checksums
    final crc8Value = crc8(lengthBytes);
    final crc32Value = crc32(payload);
    final crc32Bytes = [
      crc32Value & 0xFF,
      (crc32Value >> 8) & 0xFF,
      (crc32Value >> 16) & 0xFF,
      (crc32Value >> 24) & 0xFF,
    ];

    // Build the framed packet
    return [sof, ...lengthBytes, crc8Value, ...payload, ...crc32Bytes];
  }

  /// Parse a packet from raw bytes
  static WhoopPacket? fromBytes(List<int> bytes) {
    if (bytes.length < 8) return null;

    // Check start of frame
    if (bytes[0] != sof) return null;

    // Verify header CRC8
    final lengthBytes = bytes.sublist(1, 3);
    final expectedCrc8 = bytes[3];
    if (crc8(lengthBytes) != expectedCrc8) return null;

    // Get payload length
    final length = lengthBytes[0] | (lengthBytes[1] << 8);
    if (bytes.length < 4 + length) return null;

    // Extract payload (without CRC32)
    final payloadWithCrc = bytes.sublist(4, 4 + length);
    final payload = payloadWithCrc.sublist(0, payloadWithCrc.length - 4);

    // Verify CRC32
    final expectedCrc32 = payloadWithCrc[payload.length] |
        (payloadWithCrc[payload.length + 1] << 8) |
        (payloadWithCrc[payload.length + 2] << 16) |
        (payloadWithCrc[payload.length + 3] << 24);
    if (crc32(payload) != expectedCrc32) return null;

    // Parse packet fields
    final packetType = PacketType.fromValue(payload[0]);
    if (packetType == null) return null;

    return WhoopPacket(
      packetType: packetType,
      seq: payload[1],
      cmd: payload[2],
      data: payload.sublist(3),
    );
  }

  /// Create a "Hello Harvard" command
  static WhoopPacket helloHarvard() {
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.getHelloHarvard.value,
      data: [0x00],
    );
  }

  /// Create a "Set Clock" command with current time
  static WhoopPacket setClock() {
    final timestamp = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final timeBytes = [
      timestamp & 0xFF,
      (timestamp >> 8) & 0xFF,
      (timestamp >> 16) & 0xFF,
      (timestamp >> 24) & 0xFF,
    ];
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.setClock.value,
      data: [...timeBytes, 0, 0, 0, 0, 0], // padding
    );
  }

  /// Create an "Enter High Freq Sync" command
  static WhoopPacket enterHighFreqSync() {
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.enterHighFreqSync.value,
      data: [],
    );
  }

  /// Create a "Start History" command
  static WhoopPacket historyStart() {
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.sendHistoricalData.value,
      data: [0x00],
    );
  }

  /// Create a "History End" acknowledgment
  static WhoopPacket historyEnd(int dataValue) {
    final dataBytes = [
      dataValue & 0xFF,
      (dataValue >> 8) & 0xFF,
      (dataValue >> 16) & 0xFF,
      (dataValue >> 24) & 0xFF,
    ];
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.historicalDataResult.value,
      data: [0x01, ...dataBytes, 0, 0, 0, 0], // padding
    );
  }

  /// Create a "Get Name" command
  static WhoopPacket getName() {
    return WhoopPacket(
      packetType: PacketType.command,
      seq: 0,
      cmd: CommandNumber.getAdvertisingNameHarvard.value,
      data: [0x00],
    );
  }
}

/// Parse heart rate data from a historical data packet
HeartRateReading? parseHeartRateReading(List<int> data) {
  if (data.length < 20) return null;

  try {
    // Skip first 4 bytes (unknown)
    // Bytes 4-7: Unix timestamp (little-endian)
    final timestamp = data[4] |
        (data[5] << 8) |
        (data[6] << 16) |
        (data[7] << 24);

    // Skip 6 bytes (unknown)
    // Byte 14: BPM
    final bpm = data[14];

    // Byte 15: RR interval count
    final rrCount = data[15];

    // Bytes 16-23: Up to 4 RR intervals (16-bit little-endian each)
    final rrIntervals = <int>[];
    for (var i = 0; i < 4; i++) {
      final offset = 16 + (i * 2);
      if (offset + 1 >= data.length) break;
      final rr = data[offset] | (data[offset + 1] << 8);
      if (rr > 0) {
        rrIntervals.add(rr);
      }
    }

    // Bytes 24-27: Activity value
    final activity = data.length >= 28
        ? data[24] | (data[25] << 8) | (data[26] << 16) | (data[27] << 24)
        : 0;

    return HeartRateReading(
      timestamp: timestamp,
      bpm: bpm,
      rrIntervals: rrIntervals,
      activity: activity,
    );
  } catch (e) {
    return null;
  }
}

/// Manages communication with a Whoop device
class WhoopDevice {
  final BluetoothDevice device;
  BluetoothCharacteristic? _cmdToStrap;
  BluetoothCharacteristic? _cmdFromStrap;
  BluetoothCharacteristic? _dataFromStrap;
  BluetoothCharacteristic? _eventsFromStrap;

  final List<HeartRateReading> readings = [];
  HeartRateReading? latestReading;

  Function(HeartRateReading)? onHeartRateReceived;
  Function(String)? onStatusChanged;

  WhoopDevice(this.device);

  /// Initialize the device connection and start receiving data
  Future<void> initialize() async {
    onStatusChanged?.call('Discovering services...');

    // Discover services
    final services = await device.discoverServices();

    // Find the Whoop service
    BluetoothService? whoopService;
    for (var service in services) {
      if (service.uuid == WhoopUuids.service) {
        whoopService = service;
        break;
      }
    }

    if (whoopService == null) {
      throw Exception('Whoop service not found');
    }

    // Find characteristics
    for (var char in whoopService.characteristics) {
      if (char.uuid == WhoopUuids.cmdToStrap) {
        _cmdToStrap = char;
      } else if (char.uuid == WhoopUuids.cmdFromStrap) {
        _cmdFromStrap = char;
      } else if (char.uuid == WhoopUuids.dataFromStrap) {
        _dataFromStrap = char;
      } else if (char.uuid == WhoopUuids.eventsFromStrap) {
        _eventsFromStrap = char;
      }
    }

    if (_cmdToStrap == null || _dataFromStrap == null) {
      throw Exception('Required characteristics not found');
    }

    onStatusChanged?.call('Subscribing to notifications...');

    // Subscribe to notifications
    if (_dataFromStrap != null) {
      await _dataFromStrap!.setNotifyValue(true);
      _dataFromStrap!.onValueReceived.listen(_handleDataNotification);
    }

    if (_cmdFromStrap != null) {
      await _cmdFromStrap!.setNotifyValue(true);
      _cmdFromStrap!.onValueReceived.listen(_handleCmdNotification);
    }

    if (_eventsFromStrap != null) {
      await _eventsFromStrap!.setNotifyValue(true);
    }

    onStatusChanged?.call('Initializing device...');

    // Send initialization commands
    await _sendCommand(WhoopPacket.helloHarvard());
    await Future.delayed(const Duration(milliseconds: 100));
    await _sendCommand(WhoopPacket.setClock());
    await Future.delayed(const Duration(milliseconds: 100));
    await _sendCommand(WhoopPacket.enterHighFreqSync());
    await Future.delayed(const Duration(milliseconds: 100));

    onStatusChanged?.call('Starting data sync...');
    await _sendCommand(WhoopPacket.historyStart());

    onStatusChanged?.call('Connected - Receiving data');
  }

  void _handleDataNotification(List<int> value) {
    final packet = WhoopPacket.fromBytes(value);
    if (packet == null) return;

    if (packet.packetType == PacketType.historicalData) {
      final reading = parseHeartRateReading(packet.data);
      if (reading != null && reading.bpm > 0 && reading.bpm < 250) {
        latestReading = reading;
        readings.add(reading);
        onHeartRateReceived?.call(reading);
      }
    } else if (packet.packetType == PacketType.metadata) {
      // Handle metadata (history end marker)
      if (packet.cmd == 2 && packet.data.length >= 8) {
        // HistoryEnd - send acknowledgment
        final dataValue = packet.data[6] |
            (packet.data[7] << 8) |
            (packet.data[8] << 16) |
            (packet.data[9] << 24);
        _sendCommand(WhoopPacket.historyEnd(dataValue));
      }
    }
  }

  void _handleCmdNotification(List<int> value) {
    // Handle command responses if needed
  }

  Future<void> _sendCommand(WhoopPacket packet) async {
    if (_cmdToStrap == null) return;

    final bytes = packet.toFramedPacket();
    await _cmdToStrap!.write(
      bytes,
      withoutResponse: true,
    );
  }

  Future<void> disconnect() async {
    await device.disconnect();
  }
}
