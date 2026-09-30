package net.minecraft.client.rmc;

public final class RmcVerificationScenarios
{
    private static final String[] MOVEMENT_LINES = new String[] {
        "movement record=state step=0 total_ticks=1 ticks_run=1 pos_x=-0.001588 pos_y=0.000000 pos_z=0.045472 vel_x=-0.001588 vel_y=0.000000 vel_z=0.045472 yaw=2.000000 pitch=0.500000 network_x=0.000000 network_y=0.000000 network_z=0.000000 on_ground=true sprinting=true sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=0 packet_index=0 tick=1 packet=PlayerPositionAndLook x=-0.001588 y=0.000000 z=0.045472 yaw=2.000000 pitch=0.500000 on_ground=true",
        "movement record=state step=1 total_ticks=2 ticks_run=1 pos_x=-0.003795 pos_y=0.000000 pos_z=0.108679 vel_x=-0.002207 vel_y=0.000000 vel_z=0.063206 yaw=2.000000 pitch=0.500000 network_x=0.000000 network_y=0.000000 network_z=0.000000 on_ground=true sprinting=true sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=1 packet_index=0 tick=2 packet=PlayerPosition x=-0.003795 y=0.000000 z=0.108679 on_ground=true",
        "movement record=state step=2 total_ticks=3 ticks_run=1 pos_x=0.344123 pos_y=0.333200 pos_z=0.088308 vel_x=0.347918 vel_y=0.333200 vel_z=-0.020371 yaw=2.000000 pitch=0.500000 network_x=0.000000 network_y=0.000000 network_z=0.000000 on_ground=false sprinting=false sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=2 packet_index=0 tick=3 packet=PlayerPosition x=0.344123 y=0.333200 z=0.088308 on_ground=false",
        "movement record=state step=3 total_ticks=4 ticks_run=1 pos_x=0.635036 pos_y=0.581336 pos_z=0.081647 vel_x=0.290914 vel_y=0.248136 vel_z=-0.006661 yaw=2.000000 pitch=0.500000 network_x=0.000000 network_y=0.000000 network_z=0.000000 on_ground=false sprinting=true sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=3 packet_index=0 tick=4 packet=PlayerPosition x=0.635036 y=0.581336 z=0.081647 on_ground=false",
        "movement record=state step=4 total_ticks=5 ticks_run=1 pos_x=1.998412 pos_y=0.000000 pos_z=1.545472 vel_x=-0.001588 vel_y=0.000000 vel_z=0.045472 yaw=2.000000 pitch=0.500000 network_x=2.000000 network_y=0.000000 network_z=1.500000 on_ground=true sprinting=true sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=4 packet_index=0 tick=5 packet=PlayerPosition x=1.998412 y=0.000000 z=1.545472 on_ground=true",
        "movement record=state step=5 total_ticks=6 ticks_run=1 pos_x=1.964516 pos_y=0.000000 pos_z=1.594237 vel_x=-0.033896 vel_y=0.000000 vel_z=0.048765 yaw=2.000000 pitch=0.500000 network_x=2.000000 network_y=0.000000 network_z=1.500000 on_ground=true sprinting=true sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=5 packet_index=0 tick=6 packet=PlayerPosition x=1.964516 y=0.000000 z=1.594237 on_ground=true",
        "movement record=state step=6 total_ticks=7 ticks_run=1 pos_x=1.925699 pos_y=0.000000 pos_z=1.637126 vel_x=-0.038817 vel_y=0.000000 vel_z=0.042888 yaw=2.000000 pitch=0.500000 network_x=2.000000 network_y=0.000000 network_z=1.500000 on_ground=true sprinting=false sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=6 packet_index=0 tick=7 packet=PlayerPosition x=1.925699 y=0.000000 z=1.637126 on_ground=true",
        "movement record=state step=7 total_ticks=8 ticks_run=1 pos_x=1.902409 pos_y=0.000000 pos_z=1.662859 vel_x=-0.023290 vel_y=0.000000 vel_z=0.025733 yaw=2.000000 pitch=0.500000 network_x=2.000000 network_y=0.000000 network_z=1.500000 on_ground=true sprinting=false sneaking=false sprint_reset_ticks=0 selected_slot=0",
        "movement record=packet step=7 packet_index=0 tick=8 packet=PlayerPosition x=1.902409 y=0.000000 z=1.662859 on_ground=true"
    };

    private static final String[] COMBAT_LINES = new String[] {
        "combat record=packet step=0 packet_index=0 tick=0 packet=EntityAction entity_id=12 action=StartSprinting aux_data=0",
        "combat record=packet step=1 packet_index=1 tick=0 packet=Animation",
        "combat record=packet step=1 packet_index=2 tick=0 packet=UseEntity entity_id=44 action=Attack",
        "combat record=packet step=2 packet_index=3 tick=0 packet=PlayerBlockPlacement x=-1 y=-1 z=-1 face=255 held_item=261:1:0 cursor_x=0.000000 cursor_y=0.000000 cursor_z=0.000000",
        "combat record=state step=2 using_item=true use_ticks=2 hurt_ticks=0 health=20.000 food_level=20 saturation=5.000",
        "combat record=packet step=3 packet_index=4 tick=0 packet=PlayerDigging action=ReleaseUseItem x=0 y=0 z=0 face=0",
        "combat record=health_update step=4 health=17.000 food_level=19 saturation=4.000 hurt_feedback=true hurt_ticks=10",
        "combat record=velocity step=5 vel_x=0.200000 vel_y=0.100000 vel_z=-0.050000 simulation_events=1",
        "combat record=simulation_event step=5 event=Knockback x=0.200000 y=0.100000 z=-0.050000 resets_sprint=true",
        "combat record=final step=6 server_sprint_state=true server_sneak_state=false using_item=false health=17.000 food_level=19 saturation=4.000 hurt_ticks=10 last_vel_x=0.200000 last_vel_y=0.100000 last_vel_z=-0.050000"
    };

    private static final String[] INVENTORY_LINES = new String[] {
        "inventory record=packet step=0 packet_index=0 tick=0 packet=HeldItemChange slot=2",
        "inventory record=open_window step=1 window_id=4 updated_windows=1 slot_count=27 inventory_type=minecraft:chest",
        "inventory record=set_slot step=2 window_id=4 slot_id=13 updated_windows=1 slot_item=5:16:0",
        "inventory record=packet step=3 packet_index=1 tick=0 packet=ClickWindow window_id=4 slot_id=13 button=0 action_number=1 mode=0 clicked_item=5:16:0",
        "inventory record=pending_transaction step=3 count=1",
        "inventory record=confirm_transaction step=4 accepted=false rejected_transactions=1 ack_packets=1 pending_transactions=0",
        "inventory record=packet step=4 packet_index=2 tick=0 packet=ConfirmTransaction window_id=4 action_number=1 accepted=true",
        "inventory record=packet step=5 packet_index=3 tick=0 packet=CloseWindow window_id=4",
        "inventory record=final step=6 selected_slot=2 window_open=false carried_item=none pending_transactions=0"
    };

    private RmcVerificationScenarios()
    {
    }

    public static void main(String[] args)
    {
        emit("movement", MOVEMENT_LINES);
        emit("combat", COMBAT_LINES);
        emit("inventory", INVENTORY_LINES);
        RmcTraceLogger.closeAll();
    }

    private static void emit(String channel, String[] lines)
    {
        RmcTraceLogger.resetChannel(channel);

        for (String line : lines)
        {
            RmcTraceLogger.traceLine(channel, line);
        }
    }
}
