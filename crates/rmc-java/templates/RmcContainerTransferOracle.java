package net.minecraft.client.rmc;
import net.minecraft.init.Bootstrap;
import net.minecraft.inventory.*;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;

/** Executes real container algorithms against authored deterministic inventories. */
public final class RmcContainerTransferOracle {
    private static String stack(ItemStack s) {
        return s == null ? "-" : Item.getIdFromItem(s.getItem())+":"+s.stackSize+":"+s.getMetadata();
    }
    public static void main(String[] args) {
        Bootstrap.register();
        String[] kinds={"chest","hopper","dispenser","dropper","beacon","furnace"};
        int[] sizes={27,5,9,9,1,3};
        for(int type=0;type<kinds.length;type++) {
            int size=sizes[type];
            for(int source=0;source<size+36;source++) for(int id:new int[]{1,339,35,264,265,15,17,263})
            for(int count:new int[]{1,32}) for(int pattern=0;pattern<3;pattern++) {
                RmcMovementOracle.Player player=new RmcMovementOracle.Player(new RmcCollisionOracle.QueryWorld());
                InventoryBasic storage=new InventoryBasic("Probe",false,size);
                Container c=type==0?new ContainerChest(player.inventory,storage,player):
                    type==1?new ContainerHopper(player.inventory,storage,player):
                    type==4?new ContainerBeacon(player.inventory,storage):
                    type==5?new ContainerFurnace(player.inventory,storage):new ContainerDispenser(player.inventory,storage);
                for(int slot=0;slot<size+36;slot++) {
                    ItemStack value=pattern==0?null:new ItemStack(Item.getItemById(1),64,0);
                    if(pattern==2 && (slot==0 || slot==size+35)) value=new ItemStack(Item.getItemById(id),60,1);
                    c.getSlot(slot).putStack(value);
                }
                c.getSlot(source).putStack(new ItemStack(Item.getItemById(id),count,0));
                ItemStack result=c.slotClick(source,0,1,player);
                StringBuilder out=new StringBuilder("transfer "+kinds[type]+" "+source+" "+id+" "+count+" "+pattern+" "+stack(result));
                for(int slot=0;slot<size+36;slot++) out.append(" ").append(stack(c.getSlot(slot).getStack()));
                System.out.println(out);
            }
        }
    }
}
