# Zniffer protocol specification

## Frame format

The Zniffer protocol specifies two different frame formats: one for commands and one for radio frames.

### Commands

|  0  | 1          | 2      | 2+N     |
| --- | ---------- | ------ | ------- |
| SOF | Command ID | Length | Payload |

#### Start Of Frame (SOF) (8 bits)

For commands, the SOF must be set to 0x23.

#### Command (8 bits)

Specifies the ID of the command.

#### Length (8 bits)

The length specifies the length of the command payload.

#### Payload (Length * 8 bits)

The payload depends on the command.

List of commands TBD.

### Radio frames

|  0  | 1    | 2+3       | 3+N     |
| --- | -----| --------- | ------- |
| SOF | Type | Timestamp | Payload |

#### Start of Frame (SOF) (8 bits)

For frames, the SOF must be set to 0x21.

#### Type (8 bits)

The type specifies the type of radio frame:
1. Normal frame
2. Beam frame (seems unused?)
3. Beam start
4. Beam stop

#### Timestamp (16 bits)

TBD.

#### Payload

The payload depends on the frame type.

For normal frames the payload is specified as:

| 4                 | 5      | 6    | 7+8           | 9      | 9+N           |
| ----------------- | ------ | ---- | ------------- | ------ | ------------- |
| Channel and speed | Region | RSSI | Start of data | Length | Radio payload |

For Beam start frames the payload is specified as:

| 4                 | 5      | 6    | 7-10          |
| ----------------- | ------ | ---- | ------------- |
| Channel and speed | Region | RSSI | Radio payload |

For Beam stop frames the payload is specified as:

| 4    | 5       |
| ---- | ------- |
| RSSI | Counter |

#### Channel and speed

TBD.

#### Region

Z-Wave region.

#### RSSI

TBD.

#### Start of data

The Start of data must be set to 0x0321.

#### Length

The length of the radio payload.

#### Radio payload

The Z-Wave radio frame.

#### Counter

TBD.

# Resources

- Conversion of Markdown to Sphinx: pandoc input.md -f markdown -t rst -o output.rst
