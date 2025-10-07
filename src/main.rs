use zero2prod::configuration::get_configuration;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::startup::Application;
use zero2prod::issue

#[tokio::main]
async fn main() ->anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration.");
   let application = Application::build(configuration.clone()).await? 
   run_until_stopped();
   let application_task = tokio::spawn(application.run_until_stopped());
   let worker_task = tokio::spawn(run_worker_until_stopped(configuration.clone()));
   tokio::select!{
    o = application_task =>report_exit("API", o),
    o = worker_task =>report_exit("Background worker", o),
   }; 
    Ok(())
}

fn report_exit(task_name: &str,
outcome: Result<Result<(), impl Debug + Display>, JoinError>){
    match outcome{
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name);
        }
        Ok(Err(e)) =>{
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} has failed", task_name
            )
        }
        Err(e) =>{
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{}' task failed to complete", task_name
            )
        }
    }
}
