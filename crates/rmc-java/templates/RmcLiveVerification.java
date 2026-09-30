package net.minecraft.client.rmc;

import net.minecraft.client.Minecraft;
import net.minecraft.entity.Entity;

public final class RmcLiveVerification
{
    private static final boolean ENABLED = "true".equalsIgnoreCase(System.getProperty("rmc.verify.live", "false"));
    private static int scenarioTick = 0;
    private static boolean attackTriggered = false;
    private static boolean clickTriggered = false;
    private static boolean finished = false;

    private RmcLiveVerification()
    {
    }

    public static void tick(Minecraft mc)
    {
        if (!ENABLED || finished)
        {
            return;
        }

        if (mc == null || mc.theWorld == null || mc.thePlayer == null || mc.playerController == null || mc.getNetHandler() == null)
        {
            scenarioTick = 0;
            attackTriggered = false;
            clickTriggered = false;
            return;
        }

        scenarioTick = RmcTraceLogger.movementTick();

        if (!attackTriggered && scenarioTick >= 14)
        {
            attackTriggered = tryAttack(mc);
        }

        if (!clickTriggered && scenarioTick >= 22)
        {
            clickTriggered = tryWindowClick(mc);
        }

        if (scenarioTick >= 34)
        {
            finished = true;
            mc.shutdown();
        }
    }

    private static boolean tryAttack(Minecraft mc)
    {
        Entity entity = mc.theWorld.getEntityByID(44);

        if (entity == null)
        {
            return false;
        }

        mc.thePlayer.swingItem();
        mc.playerController.attackEntity(mc.thePlayer, entity);
        return true;
    }

    private static boolean tryWindowClick(Minecraft mc)
    {
        if (mc.thePlayer.openContainer == null || mc.thePlayer.openContainer.windowId != 4)
        {
            return false;
        }

        mc.playerController.windowClick(4, 13, 0, 0, mc.thePlayer);
        return true;
    }
}
