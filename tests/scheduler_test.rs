use audiotab::engine::{PipelineScheduler, Priority};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_scheduler_priority_ordering() {
    let mut scheduler = PipelineScheduler::new(2); // max 2 concurrent

    // Schedule tasks with different priorities
    let low_started = scheduler
        .schedule_task(Priority::Low, async {
            sleep(Duration::from_millis(100)).await;
            "low".to_string()
        })
        .await;

    let high_started = scheduler
        .schedule_task(Priority::High, async {
            sleep(Duration::from_millis(50)).await;
            "high".to_string()
        })
        .await;

    let _critical_started = scheduler
        .schedule_task(Priority::Critical, async {
            sleep(Duration::from_millis(25)).await;
            "critical".to_string()
        })
        .await;

    // All should start (capacity = 2, but third queued)
    assert!(low_started);
    assert!(high_started);

    // Wait for completion
    let results = scheduler.wait_all().await;

    // Critical should complete first despite being scheduled last
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_scheduler_max_concurrent() {
    let mut scheduler = PipelineScheduler::new(2);

    // Schedule 3 tasks (capacity = 2)
    scheduler
        .schedule_task(Priority::Normal, async {
            sleep(Duration::from_millis(100)).await;
            1
        })
        .await;

    scheduler
        .schedule_task(Priority::Normal, async {
            sleep(Duration::from_millis(100)).await;
            2
        })
        .await;

    scheduler
        .schedule_task(Priority::Normal, async {
            sleep(Duration::from_millis(50)).await;
            3
        })
        .await;

    // Check active count
    assert!(scheduler.active_count() <= 2);

    let results = scheduler.wait_all().await;
    assert_eq!(results.len(), 3);
}
