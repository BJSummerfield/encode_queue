Here's a more concise version of the overview, incorporating your additional points:

---

# Encode Queue

**Version:** 1.0  
**Author:** [Brian Summerfield](mailto:brianjsummerfield@gmail.com)

## Overview

Encode Queue is a command-line tool that manages a queue of shell commands, particularly useful for sequentially encoding files. The queue operates by reading and executing commands from the first line of a file, removing the line once the command is executed. This allows you to continue adding or editing commands even while the queue is processing. It runs as a single task, making it easy to suspend and resume as needed.

## Features

- **Add Commands**: Add shell commands to the queue.
- **Start Queue Processing**: Execute the commands in the queue sequentially.
- **List Commands**: View all commands currently in the queue.
- **Edit Queue**: Open the queue in your preferred text editor to manually modify the commands.

## Requirements

- **Rust**: You need to have Rust installed on your system to build and run this project.
- **Fish Shell**: The commands in the queue are executed using the Fish shell.

## Installation

1. **Clone the repository:**

   ```sh
   git clone https://github.com/BJSummerfield/encode_queue.git
   cd encode_queue
   ```

2. **Build the project:**

   ```sh
   cargo build --release
   ```

3. **Run the executable:**

   ```sh
   ./target/release/encode_queue
   ```

## Usage

After building the project, you can use the `encode_queue` command with the following options:

### Add a Command to the Queue

```sh
encode_queue add "your_command_here"
```

**Example:**

```sh
encode_queue add "ffmpeg -i input.mp4 -c:v libx264 output.mp4"
```

### Start Processing the Queue

```sh
encode_queue start
```

This will execute all commands in the queue sequentially, removing each command from the file after it is executed.

### List All Commands in the Queue

```sh
encode_queue ls
```

This will display all commands currently in the queue.

### Edit the Queue

```sh
encode_queue edit
```

This will open the queue file in your default editor (as specified by the `EDITOR` environment variable).

## Configuration

The queue of commands is stored in a text file located at:

```
~/.config/encode_queue/commands.txt
```

You can change the default editor by setting the `EDITOR` environment variable:

```sh
export EDITOR=nvim
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

If you have any questions, feel free to reach out to me at [brianjsummerfield@gmail.com](mailto:brianjsummerfield@gmail.com).

---

This version is more streamlined while still explaining how the queue works and the advantages of the process.
