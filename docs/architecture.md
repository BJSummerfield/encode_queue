# Architecture

## Diagram

```mermaid
graph TD
    subgraph CLI["CLI (main.rs)"]
        CliEnum["Cli enum<br/>Add / Start / Ls / Edit"]
    end

    subgraph Domain["domain/"]
        CommandText["CommandText<br/>newtype for validated commands"]
        InvalidCommandError["InvalidCommandError<br/>Empty / WhitespaceOnly"]
        QueueError["QueueError<br/>Io / Env / EditorSpawn / InvalidCommand"]
        QueueRepository["QueueRepository trait<br/>add / list / get_and_remove_first / open_in_editor"]
    end

    subgraph Adapter["adapters/"]
        FileQueue["FileQueue<br/>file-based QueueRepository impl"]
    end

    subgraph Features["features/"]
        add["add_command"]
        list["list_commands"]
        edit["edit_queue"]
        process["process_queue"]
    end

    subgraph External["External"]
        File["commands.txt"]
        Fish["fish shell"]
        Editor["$EDITOR / nvim"]
    end

    %% Dependencies
    CLI -->|"parse & dispatch"| Features
    CLI -->|"create"| FileQueue

    CommandText -->|"constructor fails with"| InvalidCommandError
    QueueError -->|"wraps"| InvalidCommandError
    QueueError -->|"wraps"| "io::Error"

    FileQueue -->|"implements"| QueueRepository
    FileQueue -->|"reads/writes"| File
    FileQueue -->|"spawns"| Editor

    add -->|"uses"| CommandText
    add -->|"calls"| QueueRepository
    list -->|"calls"| QueueRepository
    edit -->|"calls"| QueueRepository
    process -->|"calls"| QueueRepository
    process -->|"spawns"| Fish

    %% All features return QueueError
    add -->|"returns"| QueueError
    list -->|"returns"| QueueError
    edit -->|"returns"| QueueError
    process -->|"returns"| QueueError

    classDef domain fill:#2d5a73,stroke:#1a3a4a,color:#fff
    classDef adapter fill:#5a7a5a,stroke:#3a5a3a,color:#fff
    classDef feature fill:#7a5a5a,stroke:#5a3a3a,color:#fff
    classDef cli fill:#5a5a7a,stroke:#3a3a5a,color:#fff

    class CommandText,InvalidCommandError,QueueError,QueueRepository domain
    class FileQueue adapter
    class add,list,edit,process feature
    class CliEnum cli
```

## Domain Types

| Type | Purpose |
|---|---|
| `CommandText` | Newtype wrapping a validated command string. Only constructible via `CommandText::new()` or `TryFrom<&str>`. Prevents invalid commands from entering the system. |
| `InvalidCommandError` | Error returned when command validation fails. Two variants: `Empty` and `WhitespaceOnly`. |
| `QueueError` | Top-level domain error. Four variants: `Io(io::Error)`, `Env` (missing `$HOME`), `EditorSpawn(String)`, `InvalidCommand(InvalidCommandError)`. Every operation in the system returns this. |
| `QueueRepository` | Trait defining the queue interface: `add`, `list`, `get_and_remove_first`, `open_in_editor`. Domain defines it; adapters implement it. Enables mocking for tests. |

## Adapter

| Type | Purpose |
|---|---|
| `FileQueue` | Implements `QueueRepository` using `~/.config/encode_queue/commands.txt`. One command per line. |

## Key Property

Dependencies flow inward. Features depend on `QueueRepository` (trait), never on `FileQueue`. `main.rs` is the only place that knows about `FileQueue` concretely.