use proto_pdk_test_utils::*;

mod flutter_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn accepts_empty_passthrough_args() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        assert_eq!(
            plugin.pre_run(RunHook::default()).await,
            RunHookResult::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn accepts_commands_and_channel_listing() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("flutter-test").await;

        for passthrough_args in [vec!["doctor".into()], vec!["channel".into()]] {
            assert_eq!(
                plugin
                    .pre_run(RunHook {
                        passthrough_args,
                        ..RunHook::default()
                    })
                    .await,
                RunHookResult::default()
            );
        }
    }
}
