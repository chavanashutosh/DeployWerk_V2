import { toast as sonnerToast } from "sonner";

export function toastSuccess(message: string) {
  sonnerToast.success(message);
}

export function toastError(message: string) {
  sonnerToast.error(message);
}
