# Tracer Daemon Instructions

## How to check if Tracer Daemon Is Running:

```bash
$ps -e | grep tracer
```

## Individual process consumption

````rust
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
    ```
````
