# Tracer Daemon Instructions

## How to check if Tracer Daemon Is Running:

```bash
$ps -e | grep tracer
```

## Individual process consumption

```rust
// Sends current resource consumption of target processes to the server
async fn send_proc_stat(&self) -> Result<()> {
for (pid, proc) in self.seen.iter() {
let Some(p) = self.system.process(\*pid) else {
eprintln!("[{}] Process({}) wasn't found", Utc::now(), proc);
return Ok(());
};

            let attributes = json!({
                "name": format!("{} metric", proc),
                "memory_usage": p.memory(),
                "cpu_usage": p.cpu_usage(),
            });
            self.send_event(
                EventStatus::MetricEvent,
                &format!("[{}] {}({}) resources metric", Utc::now(), proc, pid),
                Some(attributes),
            )
            .await?;
        }
        Ok(())
    }

        // #[tokio::test]
    // async fn tool_finish() {
    //     // Fixed the issue by ensuring that processes are properly refreshed and removed.
    //     let mut tr = TracerClient::from_config(create_conf()).unwrap();
    //     tr.targets = vec!["sleep".to_string()];

    //     let mut cmd = std::process::Command::new("sleep")
    //         .arg("1")
    //         .spawn()
    //         .unwrap();

    //     while tr.seen.len() <= 0 {
    //         TracerClient::refresh(&mut tr);
    //         TracerClient::poll_processes(&mut tr).await.unwrap();
    //     }

    //     cmd.wait().unwrap();
    //     TracerClient::refresh(&mut tr);

    //     TracerClient::remove_completed_processes(&mut tr)
    //         .await
    //         .unwrap();

    //     assert_eq!(tr.seen.len(), 0);
    // }
```
