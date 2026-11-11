import { Toaster as Sonner } from "sonner";

export function Toaster() {
  return (
    <Sonner
      theme="dark"
      position="bottom-right"
      mobileOffset={{
        right: 16,
        bottom: "calc(4.75rem + env(safe-area-inset-bottom))",
      }}
      closeButton
      richColors
      toastOptions={{
        classNames: {
          toast: "font-sans",
        },
      }}
    />
  );
}
