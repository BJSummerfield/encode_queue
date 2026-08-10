use encode_queue::{
    adapters::FileQueue,
    features,
};
use std::fs;
use tempfile::TempDir;

fn setup_queue() -> (FileQueue, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("commands.txt");
    (FileQueue::with_path(path), dir)
}

#[test]
fn add_then_list() {
    let (mut queue, _dir) = setup_queue();
    features::add::add_command(&mut queue, "ffmpeg -i input.mp4 output.mp4").unwrap();

    let commands = queue.list().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].as_str(), "ffmpeg -i input.mp4 output.mp4");
}

#[test]
fn add_multiple_then_list_preserves_order() {
    let (mut queue, _dir) = setup_queue();
    features::add::add_command(&mut queue, "first").unwrap();
    features::add::add_command(&mut queue, "second").unwrap();
    features::add::add_command(&mut queue, "third").unwrap();

    let commands = queue.list().unwrap();
    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].as_str(), "first");
    assert_eq!(commands[1].as_str(), "second");
    assert_eq!(commands[2].as_str(), "third");
}

#[test]
fn add_empty_command_rejected() {
    let (mut queue, _dir) = setup_queue();
    let result = features::add::add_command(&mut queue, "");
    assert!(result.is_err());
}

#[test]
fn list_empty_queue() {
    let (queue, _dir) = setup_queue();
    let result = features::list::list_commands(&queue);
    assert!(result.is_ok());
}

#[test]
fn queue_persists_to_file() {
    let (mut queue, dir) = setup_queue();
    features::add::add_command(&mut queue, "persisted command").unwrap();

    // Verify file exists and contains the command
    let contents = fs::read_to_string(dir.path().join("commands.txt")).unwrap();
    assert!(contents.contains("persisted command"));
}