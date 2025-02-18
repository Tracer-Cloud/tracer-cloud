use bollard::container::{InspectContainerOptions, ListContainersOptions};
use bollard::Docker;
use std::process::Command;
use tokio::time::{sleep, Duration};

pub async fn monitor_container(docker: &Docker, container_prefix: &str) {
    let options = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };

    let containers: Vec<String> = docker
        .list_containers(Some(options))
        .await
        .expect("Failed to get containers")
        .iter()
        .filter_map(|ex| {
            ex.names.as_ref().and_then(|names| {
                names
                    .iter()
                    .find(|name| name.contains(container_prefix))
                    .map(|name| name.trim_start_matches('/').to_string())
            })
        })
        .collect();

    if containers.is_empty() {
        println!(
            "No running containers found with prefix: {}",
            container_prefix
        );
        return;
    }

    loop {
        let mut all_stopped = true;

        for container_name in &containers {
            if let Ok(container_info) = docker
                .inspect_container(container_name, Some(InspectContainerOptions::default()))
                .await
            {
                if let Some(state) = container_info.state {
                    if state.running.unwrap_or(false) {
                        all_stopped = false;
                    }
                }
            }
        }

        if all_stopped {
            break; // All containers have stopped, exit loop
        }

        sleep(Duration::from_secs(2)).await;
    }

    println!("All monitored containers have finished executing.");
}

pub async fn start_docker_compose(profile: &str) {
    let output = Command::new("docker-compose")
        .arg("--profile")
        .arg(profile)
        .arg("up")
        .arg("-d") // Detached mode
        .output()
        .expect("Failed to start Docker Compose");

    if !output.status.success() {
        eprintln!(
            "Failed to start Docker Compose: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

pub async fn end_docker_compose(profile: &str) {
    let output = Command::new("docker-compose")
        .arg("--profile")
        .arg(profile)
        .arg("down")
        .output()
        .expect("Failed to start Docker Compose");

    if !output.status.success() {
        eprintln!(
            "Failed to end Docker Compose: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
