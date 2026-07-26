import { useEffect, useRef, type ReactNode } from "react";

export interface ModalDialogProps {
  children: ReactNode;
  className?: string;
  labelledBy: string;
  onClose(): void;
}

export function ModalDialog({
  children,
  className,
  labelledBy,
  onClose,
}: ModalDialogProps) {
  const dialog = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const element = dialog.current;
    if (!element) {
      return;
    }

    try {
      element.showModal();
    } catch {
      element.setAttribute("open", "");
    }

    return () => {
      if (element.open && typeof element.close === "function") {
        element.close();
      }
    };
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`profile-dialog ${className ?? ""}`}
      aria-labelledby={labelledBy}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
        }
      }}
    >
      {children}
    </dialog>
  );
}
