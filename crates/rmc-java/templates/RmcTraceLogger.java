package net.minecraft.client.rmc;

import java.io.BufferedWriter;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.network.EnumConnectionState;

public final class RmcTraceLogger
{
    private static final Object LOCK = new Object();
    private static final Map<String, BufferedWriter> WRITERS = new HashMap<String, BufferedWriter>();
    private static final File ROOT = resolveRoot();
    private static final boolean ENABLED = !"false".equalsIgnoreCase(System.getProperty("rmc.trace.enabled", "true"));

    static
    {
        if (!ROOT.exists())
        {
            ROOT.mkdirs();
        }

        Runtime.getRuntime().addShutdownHook(new Thread("rmc-trace-close")
        {
            public void run()
            {
                closeAll();
            }
        });
    }

    private RmcTraceLogger()
    {
    }

    public static void tracePacket(String direction, EnumConnectionState state, int packetId, String rawPacketName, int length, String compression)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("packet", "packet state=" + safe(formatState(state)) + " direction=" + safe(direction) + " id=" + hex(packetId) + " name=" + safe(normalizePacketName(rawPacketName)) + " len=" + length + " compression=" + safe(compression));
    }

    public static void traceLine(String channel, String line)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine(channel, line);
    }

    public static void resetChannel(String channel)
    {
        synchronized (LOCK)
        {
            BufferedWriter writer = WRITERS.remove(channel);

            if (writer != null)
            {
                try
                {
                    writer.close();
                }
                catch (IOException ignored)
                {
                }
            }

            File file = new File(ROOT, channel + "-java.trace");

            if (file.exists())
            {
                file.delete();
            }
        }
    }

    public static void traceMovementState(int tick, double posX, double posY, double posZ, double motionX, double motionY, double motionZ, float yaw, float pitch, boolean onGround, boolean sprinting, boolean sneaking, String packet, boolean moved, boolean rotated)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("movement", "movement record=state tick=" + tick + " pos_x=" + formatDouble(posX) + " pos_y=" + formatDouble(posY) + " pos_z=" + formatDouble(posZ) + " vel_x=" + formatDouble(motionX) + " vel_y=" + formatDouble(motionY) + " vel_z=" + formatDouble(motionZ) + " yaw=" + formatFloat(yaw) + " pitch=" + formatFloat(pitch) + " on_ground=" + onGround + " sprinting=" + sprinting + " sneaking=" + sneaking + " packet=" + safe(packet) + " moved=" + moved + " rotated=" + rotated);
    }

    public static void traceMovementCorrection(double x, double y, double z, float yaw, float pitch, String flags)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("movement", "movement record=server_correction x=" + formatDouble(x) + " y=" + formatDouble(y) + " z=" + formatDouble(z) + " yaw=" + formatFloat(yaw) + " pitch=" + formatFloat(pitch) + " flags=" + safe(flags));
    }

    public static void traceCombatAction(String action, int entityId, int selectedSlot, boolean sprinting)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("combat", "combat record=" + safe(action) + " entity_id=" + entityId + " selected_slot=" + selectedSlot + " sprinting=" + sprinting);
    }

    public static void traceCombatUseItem(int selectedSlot, ItemStack stack)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("combat", "combat record=use_item selected_slot=" + selectedSlot + " item=" + safe(formatItemStack(stack)));
    }

    public static void traceCombatReleaseUseItem(int selectedSlot)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("combat", "combat record=release_use_item selected_slot=" + selectedSlot);
    }

    public static void traceCombatHealth(float health, int foodLevel, float saturation)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("combat", "combat record=health_update health=" + formatFloat(health) + " food_level=" + foodLevel + " saturation=" + formatFloat(saturation));
    }

    public static void traceCombatVelocity(int entityId, double motionX, double motionY, double motionZ, boolean localPlayer)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("combat", "combat record=velocity entity_id=" + entityId + " local_player=" + localPlayer + " x=" + formatDouble(motionX) + " y=" + formatDouble(motionY) + " z=" + formatDouble(motionZ));
    }

    public static void traceInventoryHeldItem(int slot)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=held_item slot=" + slot);
    }

    public static void traceInventoryOpenWindow(int windowId, String inventoryType, int slotCount, String title)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=open_window window_id=" + windowId + " inventory_type=" + safe(inventoryType) + " slot_count=" + slotCount + " title=" + safe(title));
    }

    public static void traceInventorySetSlot(int windowId, int slotId, ItemStack stack)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=set_slot window_id=" + windowId + " slot_id=" + slotId + " item=" + safe(formatItemStack(stack)));
    }

    public static void traceInventoryWindowItems(int windowId, int slotCount)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=window_items window_id=" + windowId + " slot_count=" + slotCount);
    }

    public static void traceInventoryCloseWindow(int windowId)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=close_window window_id=" + windowId);
    }

    public static void traceInventoryClick(int windowId, int slotId, int button, int mode, int actionNumber, ItemStack stack)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=click_window window_id=" + windowId + " slot_id=" + slotId + " button=" + button + " mode=" + mode + " action_number=" + actionNumber + " item=" + safe(formatItemStack(stack)));
    }

    public static void traceInventoryConfirm(int windowId, int actionNumber, boolean accepted, boolean ackSent)
    {
        if (!ENABLED)
        {
            return;
        }

        writeLine("inventory", "inventory record=confirm_transaction window_id=" + windowId + " action_number=" + actionNumber + " accepted=" + accepted + " ack_sent=" + ackSent);
    }

    public static void closeAll()
    {
        synchronized (LOCK)
        {
            for (BufferedWriter writer : WRITERS.values())
            {
                try
                {
                    writer.close();
                }
                catch (IOException ignored)
                {
                }
            }

            WRITERS.clear();
        }
    }

    public static String normalizePacketName(String rawPacketName)
    {
        if (rawPacketName == null)
        {
            return "Unknown";
        }

        String name = rawPacketName;
        int innerIndex = name.lastIndexOf('$');

        if (innerIndex >= 0 && innerIndex + 1 < name.length())
        {
            name = name.substring(innerIndex + 1);
        }

        name = name.replaceFirst("^[A-Z][0-9A-F]{2}Packet", "");
        name = name.replaceFirst("^Packet", "");

        StringBuilder builder = new StringBuilder();

        for (int index = 0; index < name.length(); ++index)
        {
            char character = name.charAt(index);

            if (Character.isUpperCase(character) && index > 0)
            {
                builder.append('_');
            }

            builder.append(character);
        }

        if (builder.length() == 0)
        {
            return "Unknown";
        }

        return safe(builder.toString());
    }

    private static File resolveRoot()
    {
        String value = System.getProperty("rmc.trace.dir");

        if (value == null || value.length() == 0)
        {
            value = "verification/java";
        }

        return new File(value);
    }

    private static String formatState(EnumConnectionState state)
    {
        if (state == null)
        {
            return "Unknown";
        }

        String value = state.name().toLowerCase();
        return Character.toUpperCase(value.charAt(0)) + value.substring(1);
    }

    private static String formatItemStack(ItemStack stack)
    {
        if (stack == null || stack.getItem() == null)
        {
            return "none";
        }

        return Item.getIdFromItem(stack.getItem()) + ":" + stack.stackSize + ":" + stack.getMetadata();
    }

    private static String formatDouble(double value)
    {
        return String.format(java.util.Locale.US, "%.6f", value);
    }

    private static String formatFloat(float value)
    {
        return String.format(java.util.Locale.US, "%.6f", value);
    }

    private static String hex(int value)
    {
        return String.format(java.util.Locale.US, "0x%02X", Integer.valueOf(value));
    }

    private static String safe(String value)
    {
        if (value == null)
        {
            return "none";
        }

        StringBuilder builder = new StringBuilder(value.length());

        for (int index = 0; index < value.length(); ++index)
        {
            char character = value.charAt(index);

            if (Character.isLetterOrDigit(character) || character == '_' || character == '-' || character == '.' || character == ':')
            {
                builder.append(character);
            }
            else
            {
                builder.append('_');
            }
        }

        return builder.toString();
    }

    private static void writeLine(String channel, String line)
    {
        synchronized (LOCK)
        {
            BufferedWriter writer = WRITERS.get(channel);

            if (writer == null)
            {
                File file = new File(ROOT, channel + "-java.trace");

                try
                {
                    File parent = file.getParentFile();

                    if (parent != null && !parent.exists())
                    {
                        parent.mkdirs();
                    }

                    writer = new BufferedWriter(new FileWriter(file, true));
                    WRITERS.put(channel, writer);
                }
                catch (IOException error)
                {
                    throw new RuntimeException("failed to open Java trace file " + file, error);
                }
            }

            try
            {
                writer.write(line);
                writer.newLine();
                writer.flush();
            }
            catch (IOException error)
            {
                throw new RuntimeException("failed to write Java trace line", error);
            }
        }
    }
}
