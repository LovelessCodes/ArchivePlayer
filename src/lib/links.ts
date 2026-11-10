import { toast } from "sonner";
import { api } from "@/lib/api";

export async function openExternal(url: string) {
  try {
    await api.openExternal(url);
  } catch (error) {
    toast.error(error instanceof Error ? error.message : "Could not open the link.");
  }
}
