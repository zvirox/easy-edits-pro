import { useContext } from "react";
import { ProjectContext } from "./ProjectProvider";

export function useProject() {
  const project = useContext(ProjectContext);
  if (!project) throw new Error("useProject must be used within ProjectProvider");
  return project;
}
