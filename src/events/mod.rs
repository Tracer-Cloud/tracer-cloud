pub async fn event_pipeline_run_start_new() -> Result<()> {
    println!("Starting new pipeline...");
    let config = TracerAppConfig::load_config()?;

    metrics().await?;
    pipeline_new_run(&config, "[CLI] Starting pipeline run").await?;
    println!("Started pipeline run successfully...");

    Ok(())
}

async fn event_pipeline_run_end() -> Result<()> {
    println!("Ending tracer session...");
    let config = TracerAppConfig::load_config()?;

    metrics().await?;
    pipeline_finish_run(&config).await?;
    Ok(())
}
