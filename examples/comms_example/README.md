# Comms Example

This is a simple example demonstrating the `astraos-comms` communication system.

## Features demonstrated

- Creating and managing topics
- Subscribing to topics
- Publishing messages with different payload types (Text, JSON, Binary)
- Handling different QoS levels (AtMostOnce, AtLeastOnce, ExactlyOnce)
- Error handling

## Compilation

```bash
cd /home/a/ai-stack/os
cargo build --examples
```

## Running the example

```bash
cargo run --example comms_example
```

## What to expect

The example will:
1. Initialize a topic manager
2. Subscribe to the "example/messages" topic
3. Publish three different messages (text, JSON, and binary)
4. List available topics
5. Show topic information including subscriber count
6. Complete successfully

Output will show logging messages for each operation.