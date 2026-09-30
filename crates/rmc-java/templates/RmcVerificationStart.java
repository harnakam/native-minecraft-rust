package net.minecraft.client.rmc;

import java.util.Arrays;
import net.minecraft.client.main.Main;

public final class RmcVerificationStart
{
    private RmcVerificationStart()
    {
    }

    public static void main(String[] args)
    {
        Main.main(concat(new String[] {
            "--version",
            "mcp",
            "--accessToken",
            System.getProperty("rmc.verify.access_token", "0"),
            "--assetsDir",
            "assets",
            "--assetIndex",
            "1.8",
            "--userProperties",
            "{}",
            "--username",
            System.getProperty("rmc.verify.username", "RmcJavaVerify"),
            "--server",
            System.getProperty("rmc.verify.host", "127.0.0.1"),
            "--port",
            System.getProperty("rmc.verify.port", "25570"),
            "--width",
            System.getProperty("rmc.verify.width", "854"),
            "--height",
            System.getProperty("rmc.verify.height", "480")
        }, args));
    }

    private static <T> T[] concat(T[] first, T[] second)
    {
        T[] result = Arrays.copyOf(first, first.length + second.length);
        System.arraycopy(second, 0, result, first.length, second.length);
        return result;
    }
}
