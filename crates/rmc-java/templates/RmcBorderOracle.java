import net.minecraft.world.border.WorldBorder;
import java.lang.reflect.Field;
public class RmcBorderOracle {
    public static void main(String[] args) throws Exception {
        Field start = WorldBorder.class.getDeclaredField("startTime");
        Field end = WorldBorder.class.getDeclaredField("endTime");
        start.setAccessible(true); end.setAccessible(true);
        for (double from : new double[]{16,100,60000000}) {
            for (double to : new double[]{4,32,100}) {
                for (long duration : new long[]{3,1000,1000000}) {
                    for (long elapsed : new long[]{0,1,2,500,999,1000,1000000}) {
                        WorldBorder border = new WorldBorder();
                        border.setTransition(from,to,duration);
                        long origin=System.currentTimeMillis()-elapsed;
                        start.setLong(border,origin); end.setLong(border,origin+duration);
                        long before=System.currentTimeMillis()-origin;
                        double diameter=border.getDiameter();
                        long after=System.currentTimeMillis()-origin;
                        System.out.println(from+" "+to+" "+duration+" "+before+" "+after+" "+Long.toUnsignedString(Double.doubleToRawLongBits(diameter)));
                    }
                }
            }
        }
    }
}
