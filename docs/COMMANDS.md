# Command reference

Generated from the command catalogue. Do not edit by hand.

213 commands.

## File

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Relink Missing Media | `project.relink_media` | shipped |  | `{"media_id":int,"path":path}` |
| Inspect Project | `project.inspect` | shipped |  | `{}` |
| Project Status | `project.status` | shipped |  | `{}` |
| List Media | `media.list` | shipped |  | `{}` |
| New Project | `project.new` | shipped | Ctrl+N | `{"name":str?,"sequence_name":str?,"width":int?,"height":int?,"frame_rate":str?}` |
| Open Project | `file.open` | shipped | Ctrl+O | `{"path":path}` |
| Save Project | `file.save` | shipped | Ctrl+S | `{"path":path}` |
| Import Media… | `project.add_media` | shipped | Ctrl+I | `{"name":str,"path":path,"kind":str?,"duration":ticks?,"width":int?,"height":int?,"fps":num?,"audio_channels":int?,"codec":str?,"bin_id":int?,"color_label":str?,"tags":[str]?,"title":obj?,"waveform":[int]?}` |
| Import CMX 3600 EDL | `interop.import_edl` | shipped |  | `{"edl":str,"project_name":str?}` |
| Import Final Cut Pro XML | `interop.import_fcpxml` | shipped |  | `{"xml":str}` |
| Import Premiere Pro XML | `interop.import_premiere_xml` | shipped |  | `{"xml":str}` |
| Create Media Bin | `media.create_bin` | shipped |  | `{"name":str,"parent_id":int?,"color":str?}` |
| Rename Media Bin | `media.rename_bin` | shipped |  | `{"bin_id":int,"name":str}` |
| Delete Media Bin | `media.delete_bin` | shipped |  | `{"bin_id":int}` |
| Check Missing Media | `media.check_missing` | shipped |  | `{}` |
| Import OpenTimelineIO | `interop.import_otio` | shipped |  | `{"content":str,"project_name":str?}` |
| Read Preferences | `prefs.get` | shipped |  | `{"key":str?}` |
| Write Preferences | `prefs.set` | shipped |  | `{"values":obj}` |
| Reset Preferences | `prefs.reset` | shipped |  | `{}` |

## Edit

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Undo | `edit.undo` | shipped | Ctrl+Z | `{}` |
| Redo | `edit.redo` | shipped | Ctrl+Shift+Z | `{}` |
| Cut | `edit.cut` | shipped | Ctrl+X | `{}` |
| Copy | `edit.copy` | shipped | Ctrl+C | `{}` |
| Paste | `edit.paste` | shipped | Ctrl+V | `{}` |
| Delete Selected | `edit.delete` | shipped |  | `{}` |
| Select All Clips | `edit.select_all` | shipped | Ctrl+A | `{}` |
| Deselect All | `edit.deselect_all` | shipped | Ctrl+Shift+A | `{}` |
| Select Clip at Playhead | `edit.select_clip_at_playhead` | shipped |  | `{}` |
| Ripple Delete Audio Silence | `speech.ripple_delete_silence` | shipped |  | `{"peaks":[int],"options":obj?}` |
| Ripple Delete Filler Words | `speech.ripple_delete_fillers` | shipped |  | `{"segments":[obj],"custom_fillers":[str]?}` |
| Ripple Delete Time Ranges | `timeline.ripple_delete_ranges` | shipped |  | `{"ranges":obj}` |
| Ripple Delete Timeline Gap | `timeline.ripple_delete_gap` | shipped |  | `{"track_id":int?,"at_tick":obj}` |
| Preferences… | `edit.preferences` | shipped | Ctrl+, | `{}` |
| Begin Undo Gesture | `edit.begin_gesture` | shipped |  | `{"label":str?}` |
| End Undo Gesture | `edit.end_gesture` | shipped |  | `{}` |
| Ripple Delete Selected Clips | `timeline.ripple_delete_batch` | shipped |  | `{"clips":[obj],"ripple":bool?}` |
| Insert Source Clip | `timeline.insert_clip` | shipped |  | `{"media_id":int,"track_id":int?,"at":obj?,"source_in":obj?,"duration":obj?}` |
| Overwrite Source Clip | `timeline.overwrite_clip` | shipped |  | `{"media_id":int,"track_id":int?,"at":obj?,"source_in":obj?,"duration":obj?}` |
| List Shortcut Overrides | `shortcuts.list` | shipped |  | `{}` |
| Write Shortcut Overrides | `shortcuts.set` | planned |  | `{"shortcuts":obj}` |
| Reset Shortcut Overrides | `shortcuts.reset` | shipped |  | `{}` |
| Import Shortcut Overrides | `shortcuts.import` | shipped |  | `{"shortcuts":obj}` |

## Clip

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Remove Clip | `timeline.remove_clip` | shipped |  | `{"track_id":int,"clip_index":int}` |
| Split Clip at Playhead | `timeline.split_clip` | shipped |  | `{"track_id":int,"clip_index":int,"at":obj}` |
| Ripple Delete | `timeline.ripple_delete` | shipped | Shift+Delete | `{"track_id":int,"clip_index":int}` |
| Set Clip Color | `clip.set_color` | shipped |  | `{"track_id":int,"clip_index":int,"exposure":num,"contrast":num,"saturation":num,"hue":num?}` |
| Set Keyframe | `clip.set_keyframe` | shipped |  | `{"track_id":int,"clip_index":int,"property":obj,"time":obj,"value":num,"interpolation":obj?,"handle_in_x":num?,"handle_in_y":num?,"handle_out_x":num?,"handle_out_y":num?}` |
| Remove Keyframe | `clip.remove_keyframe` | shipped |  | `{"track_id":int,"clip_index":int,"property":obj,"time":obj}` |
| Add Clip Effect | `clip.add_effect` | shipped |  | `{"track_id":int,"clip_index":int,"kind":obj,"name":str?}` |
| Remove Clip Effect | `clip.remove_effect` | shipped |  | `{"track_id":int,"clip_index":int,"effect_id":int}` |
| Update Clip Effect | `clip.update_effect` | shipped |  | `{"track_id":int,"clip_index":int,"effect_id":int,"payload":obj}` |
| Toggle Clip Effect | `clip.toggle_effect` | shipped |  | `{"track_id":int,"clip_index":int,"effect_id":int,"enabled":bool?}` |
| Reorder Clip Effects | `clip.reorder_effects` | shipped |  | `{"track_id":int,"clip_index":int,"from_index":int,"to_index":int}` |
| Speed/Duration… | `timeline.rate_stretch` | shipped |  | `{"track_id":int,"clip_index":int,"new_duration":obj?,"speed":num?,"linked":bool?}` |
| Link Clips | `timeline.link_clips` | shipped | Ctrl+L | `{"clips":[obj]}` |
| Unlink Clips | `timeline.unlink_clips` | shipped | Ctrl+Shift+L | `{"clips":[obj]}` |
| Set Color Wheels | `clip.set_color_wheels` | shipped |  | `{"track_id":int,"clip_index":int,"wheels":obj}` |
| Create Multicam Clip | `multicam.create` | shipped |  | `{"name":str,"angles":[obj],"sync_mode":obj?}` |
| Switch Multicam Angle | `multicam.switch_angle` | shipped |  | `{"track_id":int,"clip_index":int,"angle_id":int}` |
| Sync Multicam Angles | `multicam.sync` | shipped |  | `{"multicam_id":int,"sync_mode":obj,"offsets":obj?}` |
| Set Motion Tracking & Stabilization | `clip.set_motion_tracking` | shipped |  | `{"track_id":int,"clip_index":int,"tracking":obj}` |
| Pin Clip to Motion Track | `clip.pin_to_track` | shipped |  | `{"track_id":int,"clip_index":int,"pinned_clip_id":int?}` |
| Set Optical Flow Speed Ramp | `clip.set_speed_ramp` | shipped |  | `{"track_id":int,"clip_index":int,"speed_ramp":obj}` |
| Set Dynamic Shutter Motion Blur | `clip.set_motion_blur` | shipped |  | `{"track_id":int,"clip_index":int,"motion_blur":obj}` |
| Create Compound Clip | `timeline.create_compound_clip` | shipped | Ctrl+G | `{"name":str?,"target_track_id":int?,"clips":[obj]}` |
| Decompose Compound Clip in Place | `timeline.decompose_compound_clip` | shipped | Ctrl+Shift+G | `{"track_id":int,"clip_index":int}` |
| Move Media to Bin | `media.move_to_bin` | shipped |  | `{"media_id":int,"bin_id":int?}` |
| Update Media Properties | `media.update_properties` | shipped |  | `{"media_id":int,"name":str?,"color_label":str?,"tags":[str]?,"comment":str?,"scene":str?,"take":str?}` |
| Update Clip RGB Curves | `clip.update_rgb_curves` | shipped |  | `{"track_id":int,"clip_index":int,"curves":obj}` |
| Update Clip HSL Qualifier | `clip.update_hsl_qualifier` | shipped |  | `{"track_id":int,"clip_index":int,"qualifier":obj}` |
| Add Custom GLSL Shader | `clip.add_custom_shader` | shipped |  | `{"track_id":int,"clip_index":int,"name":str?,"shader_code":str?,"preset_id":str?,"uniforms":obj?}` |
| Update Custom GLSL Shader | `clip.update_custom_shader` | shipped |  | `{"track_id":int,"clip_index":int,"effect_id":int,"name":str?,"shader_code":str?,"preset_id":str?,"uniforms":obj?}` |
| Slip Clip Source | `timeline.slip_clip` | shipped |  | `{"track_id":int,"clip_index":int,"delta":obj}` |
| Inspect Edit Point | `timeline.inspect` | shipped |  | `{"track_id":int?,"clip_index":int?,"at":obj?}` |
| Inspect Trim Point | `trim.inspect` | shipped |  | `{"track_id":int,"clip_index":int,"edge":"start"\|"end"}` |

## Sequence

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Go to Sequence Start | `playhead.start` | shipped | Home | `{}` |
| Go to Sequence End | `playhead.end` | shipped | End | `{}` |
| Next Edit Point | `playhead.next_edit` | shipped | Down | `{}` |
| Previous Edit Point | `playhead.previous_edit` | shipped | Up | `{}` |
| Go to In Point | `playhead.go_to_in` | shipped | Shift+I | `{}` |
| Go to Out Point | `playhead.go_to_out` | shipped | Shift+O | `{}` |
| Go to Selected Clip Start | `playhead.go_to_clip_start` | shipped | Shift+Home | `{}` |
| Go to Selected Clip End | `playhead.go_to_clip_end` | shipped | Shift+End | `{}` |
| Go to Timecode… | `playhead.go_to_timecode` | shipped |  | `{}` |
| Step Back One Frame | `playhead.nudge_back_1` | shipped | Alt+Left | `{}` |
| Step Forward One Frame | `playhead.nudge_forward_1` | shipped | Alt+Right | `{}` |
| Step Back Five Frames | `playhead.nudge_back_5` | shipped | Alt+Shift+Left | `{}` |
| Step Forward Five Frames | `playhead.nudge_forward_5` | shipped | Alt+Shift+Right | `{}` |
| Enter Trim Mode | `trim.enter` | shipped | Ctrl+Alt+E | `{"edge":"start"\|"end"?}` |
| Exit Trim Mode | `trim.exit` | shipped |  | `{}` |
| Commit Trim | `trim.commit` | shipped |  | `{}` |
| Cancel Trim | `trim.cancel` | shipped |  | `{}` |
| Add Clip to Timeline | `timeline.add_clip` | shipped |  | `{"media_id":int,"track_id":int?,"timeline_start":obj?,"source_in":obj?,"duration":obj?}` |
| Move Clip | `timeline.move_clip` | shipped |  | `{"track_id":int,"clip_index":int,"target_track_id":int?,"timeline_start":obj}` |
| Trim Clip Edge | `timeline.trim_clip` | shipped |  | `{"track_id":int,"clip_index":int,"edge":"start"\|"end","delta":obj,"ripple":bool?}` |
| Update Transition | `timeline.update_transition` | shipped |  | `{"track_id":int,"transition_id":int,"kind":obj?,"duration":obj?,"cut_at":obj?}` |
| Set Track Locked | `track.set_locked` | shipped |  | `{"track_id":int,"locked":bool}` |
| Add Transition | `timeline.add_transition` | shipped | Ctrl+D | `{"track_id":int,"cut_at":obj,"duration":obj?,"kind":obj?}` |
| Remove Transition | `timeline.remove_transition` | shipped |  | `{"track_id":int,"transition_id":int}` |
| Add Title Clip | `timeline.add_title` | shipped | Ctrl+T | `{"text":str,"subtitle":str?,"style":str?,"duration":obj?,"track_id":int?,"timeline_start":obj?}` |
| Rolling Edit | `timeline.roll_edit` | shipped | N | `{"track_id":int,"left_clip_index":int,"delta":obj}` |
| Slide Edit | `timeline.slide_clip` | shipped | U | `{"track_id":int,"clip_index":int,"delta":obj}` |
| Move Clips Batch | `timeline.move_clips_batch` | shipped |  | `{"moves":[obj]}` |
| New Sequence | `sequence.create` | shipped |  | `{"name":str?}` |
| Duplicate Sequence | `sequence.duplicate` | shipped |  | `{"sequence_id":int?,"name":str?}` |
| Rename Sequence… | `sequence.rename` | shipped |  | `{"sequence_id":int?,"name":str}` |
| Delete Sequence | `sequence.delete` | shipped |  | `{"sequence_id":int?}` |
| Switch Active Sequence | `project.switch_sequence` | shipped |  | `{"sequence_id":int}` |
| Add Timeline Track | `timeline.add_track` | shipped | Ctrl+Shift+T | `{"kind":obj?,"name":str?}` |
| Remove Timeline Track | `timeline.remove_track` | shipped |  | `{"track_id":int}` |
| Create Track Group | `timeline.create_track_group` | shipped |  | `{"name":str,"color":str?}` |
| Assign Track to Group | `timeline.assign_track_group` | shipped |  | `{"track_id":int,"group_id":int?}` |
| Toggle Track Group Collapsed | `timeline.toggle_track_group_collapsed` | shipped |  | `{"group_id":int}` |
| Delete Track Group | `timeline.delete_track_group` | shipped |  | `{"group_id":int}` |
| Set Track Group Properties | `timeline.set_track_group_properties` | shipped |  | `{"group_id":int,"name":str?,"color":str?,"muted":bool?,"solo":bool?,"locked":bool?}` |
| Lift | `sequence.lift` | shipped | ; | `{"ranges":obj}` |
| Extract | `sequence.extract` | shipped | ' | `{"ranges":obj}` |

## Markers

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Add Marker | `marker.add` | shipped | M | `{"at":ticks,"duration":ticks?,"name":str?,"kind":str?,"color":str?,"note":str?}` |
| Add Range Marker | `marker.add_range` | shipped | Shift+M | `{"start":ticks,"end":ticks,"name":str?,"kind":str?,"color":str?,"note":str?}` |
| Edit Marker | `marker.update` | shipped |  | `{"id":int,"at":ticks?,"duration":ticks?,"name":str?,"kind":str?,"color":str?,"note":str?}` |
| Delete Marker | `marker.remove` | shipped |  | `{"id":int}` |
| Clear All Markers | `marker.clear` | shipped |  | `{}` |
| Go to Next Marker | `marker.next` | shipped | Shift+Right | `{"from":ticks}` |
| Go to Previous Marker | `marker.previous` | shipped | Shift+Left | `{"from":ticks}` |
| Split Markers at Playhead | `marker.split` | shipped |  | `{"at":ticks}` |
| Ripple Markers | `marker.ripple` | shipped |  | `{"from":ticks,"delta":ticks}` |
| Markers Panel | `marker.panel` | shipped |  | `{}` |
| Export Markers (CSV) | `marker.export_csv` | shipped |  | `{}` |
| Import Markers (CSV) | `marker.import_csv` | shipped |  | `{"csv":str,"replace":bool?}` |
| Set In Point | `timeline.set_mark_in` | shipped | I | `{"at":obj}` |
| Set Out Point | `timeline.set_mark_out` | shipped | O | `{"at":obj}` |
| Clear In/Out Points | `timeline.clear_marks` | shipped |  | `{}` |

## Graphics

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Add Caption Track | `caption.track_add` | shipped |  | `{"name":str?,"language":str?}` |
| Remove Caption Track | `caption.track_remove` | shipped |  | `{"id":int}` |
| Rename Caption Track | `caption.track_rename` | shipped |  | `{"id":int,"name":str}` |
| Add Caption Cue | `caption.cue_add` | shipped |  | `{"track_id":int?,"start_ticks":obj,"end_ticks":obj,"text":str,"speaker":str?,"style_id":int?}` |
| Edit Caption Cue | `caption.cue_update` | shipped |  | `{"track_id":int,"id":int,"start_ticks":obj?,"end_ticks":obj?,"text":str?,"speaker":str?,"style_id":int?}` |
| Delete Caption Cue | `caption.cue_remove` | shipped |  | `{"track_id":int,"id":int}` |
| Nudge Caption Timing | `caption.cue_nudge` | shipped |  | `{"track_id":int,"id":int,"delta_ticks":int}` |
| Import Captions (SRT/VTT) | `caption.import` | shipped |  | `{"track_id":int?,"content":str}` |
| Export Captions (SRT/VTT) | `caption.export` | shipped |  | `{"track_id":int?,"format":str?}` |
| Set Caption Style | `caption.style_set` | planned |  | `{"track_id":int,"id":int?,"name":str?,"font_family":str?,"font_size":num?,"fill_color":str?,"stroke_color":str?,"stroke_width":num?,"background_color":str?}` |
| Captions Panel | `caption.panel` | shipped |  | `{}` |
| Update Title Properties | `title.update` | shipped |  | `{"media_id":int,"text":str?,"subtitle":str?,"style":str?,"font_size":num?,"color":str?,"bg_color":str?}` |
| Add Subtitle | `subtitle.add` | shipped |  | `{"start_ticks":obj,"end_ticks":obj,"text":str}` |
| Remove Subtitle | `subtitle.remove` | shipped |  | `{"id":int}` |
| Update Subtitle | `subtitle.update` | shipped |  | `{"id":int,"start_ticks":obj?,"end_ticks":obj?,"text":str?}` |
| Clear Subtitles | `subtitle.clear` | shipped |  | `{}` |
| Essential Graphics Panel | `graphic.panel` | shipped |  | `{}` |
| List Fonts | `fonts.list` | shipped |  | `{}` |
| Add Graphic | `graphic.add` | shipped |  | `{"kind":"text\|rectangle\|ellipse\|polygon\|adjustment","name":str?,"start_ticks":obj?,"duration_ticks":obj?,"text":str?}` |
| Remove Graphic | `graphic.remove` | shipped |  | `{"id":int}` |
| Duplicate Graphic | `graphic.duplicate` | shipped |  | `{"id":int}` |
| Set Graphic Flags | `graphic.set_flags` | shipped |  | `{"id":int,"visible":bool?,"locked":bool?}` |
| Set Graphic Transform | `graphic.set_transform` | shipped |  | `{"id":int,"x":num?,"y":num?,"width":num?,"height":num?,"scale":num?,"rotation":num?,"anchor_x":num?,"anchor_y":num?,"opacity":num?}` |
| Set Graphic Text | `graphic.set_text` | shipped |  | `{"id":int,"text":str?,"font_family":str?,"font_size":num?,"tracking":num?,"leading":num?,"align":str?,"vertical":bool?}` |
| Set Graphic Fill | `graphic.set_fill` | shipped |  | `{"id":int,"color":str?,"opacity":num?}` |
| Set Graphic Stroke | `graphic.set_stroke` | shipped |  | `{"id":int,"color":str?,"width":num?}` |
| Set Graphic Shadow | `graphic.set_shadow` | shipped |  | `{"id":int,"color":str?,"blur":num?,"offset_x":num?,"offset_y":num?}` |
| Align Graphics | `graphic.align` | shipped |  | `{"ids":[int],"mode":"left\|hcenter\|right\|top\|vcenter\|bottom"}` |
| Distribute Graphics | `graphic.distribute` | shipped |  | `{"ids":[int],"axis":"horizontal\|vertical"}` |
| Arrange Graphic Order | `graphic.arrange` | shipped |  | `{"id":int,"order":"front\|back\|forward\|backward"}` |
| Save Graphic Template | `graphic.template_save` | shipped |  | `{"graphic_id":int,"name":str}` |
| Apply Graphic Template | `graphic.template_apply` | shipped |  | `{"template_id":int,"start_ticks":obj?}` |

## Audio

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Set Track Gain | `track.set_gain_db` | shipped |  | `{"track_id":int,"gain_db":num}` |
| Set Track Pan | `track.set_pan` | shipped |  | `{"track_id":int,"pan":num}` |
| Set Track Muted | `track.set_muted` | shipped |  | `{"track_id":int,"muted":bool}` |
| Set Track Solo | `track.set_solo` | shipped |  | `{"track_id":int,"solo":bool}` |
| Set Track Audio Processing | `track.set_audio_processing` | shipped |  | `{"track_id":int,"eq_enabled":bool?,"eq_low_gain":num?,"eq_mid_gain":num?,"eq_high_gain":num?,"comp_enabled":bool?,"comp_threshold":num?,"comp_ratio":num?,"comp_attack":num?,"comp_release":num?,"denoise_enabled":bool?,"denoise_amount":num?}` |
| Set Track Bus Route | `track.set_bus_route` | shipped |  | `{"track_id":int,"bus":obj}` |
| Set Track Automation | `track.set_automation` | shipped |  | `{"track_id":int,"automation":obj}` |
| Set Track Automation Mode | `track.set_automation_mode` | shipped |  | `{"track_id":int,"mode":"off\|read\|touch\|latch\|write"}` |
| Set Track Automation Interpolation | `track.set_automation_interpolation` | shipped |  | `{"track_id":int,"interpolation":"linear\|constant\|smooth"}` |
| Add Track Automation Point | `track.add_automation_point` | shipped |  | `{"track_id":int,"lane":"volume\|pan","time":ticks,"value":num}` |
| Remove Track Automation Point | `track.remove_automation_point` | shipped |  | `{"track_id":int,"lane":"volume\|pan","time":ticks}` |
| Clear Track Automation | `track.clear_automation` | shipped |  | `{"track_id":int,"lane":"volume\|pan"?}` |
| Set Audio Bus Gain | `bus.set_gain_db` | shipped |  | `{"target":obj,"gain_db":num}` |
| Transcribe Speech to Subtitles | `speech.transcribe_to_subtitles` | shipped |  | `{"segments":[obj],"replace_existing":bool?}` |

## Effects

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Add Node to Graph | `nodegraph.add_node` | planned |  | `{"track_id":int?,"clip_index":int?,"kind":obj,"title":str?,"pos_x":num?,"pos_y":num?,"params":any?}` |
| Remove Node from Graph | `nodegraph.remove_node` | planned |  | `{"track_id":int?,"clip_index":int?,"node_id":int}` |
| Connect Graph Sockets | `nodegraph.connect` | planned |  | `{"track_id":int?,"clip_index":int?,"from_node":int,"from_socket":str,"to_node":int,"to_socket":str}` |
| Disconnect Graph Sockets | `nodegraph.disconnect` | planned |  | `{"track_id":int?,"clip_index":int?,"from_node":int,"from_socket":str,"to_node":int,"to_socket":str}` |
| Update Node Properties | `nodegraph.update_node` | planned |  | `{"track_id":int?,"clip_index":int?,"node_id":int,"pos_x":num?,"pos_y":num?,"enabled":bool?,"params":any?}` |
| Add Roto-Spline Mask | `nodegraph.add_mask` | planned |  | `{"track_id":int?,"clip_index":int?,"mask":obj}` |

## View

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Snap to Frame | `view.toggle_snap` | shipped | S | `{}` |
| Zoom In | `view.zoom_in` | shipped | Ctrl++ | `{}` |
| Zoom Out | `view.zoom_out` | shipped | Ctrl+- | `{}` |
| Reset Timeline Zoom | `view.zoom_reset` | shipped | Ctrl+0 | `{}` |
| Toggle Full Screen | `view.fullscreen` | shipped | F11 | `{}` |
| Appearance: Full Glass | `view.glass_full` | shipped |  | `{}` |
| Appearance: Lite Glass | `view.glass_lite` | shipped |  | `{}` |
| Appearance: No Glass | `view.glass_off` | shipped |  | `{}` |
| Theme: Dark | `view.theme_dark` | shipped |  | `{}` |
| Theme: Light | `view.theme_light` | shipped |  | `{}` |
| Reduce Motion | `view.reduced_motion` | shipped |  | `{}` |
| Media Pool | `view.panel_media` | shipped |  | `{}` |
| Inspector | `view.panel_inspector` | shipped |  | `{}` |

## Window

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Workspace: Media | `workspace.media` | shipped | Shift+1 | `{}` |
| Workspace: Edit | `workspace.edit` | shipped | Shift+2 | `{}` |
| Workspace: Fusion | `workspace.fusion` | shipped | Shift+3 | `{}` |
| Workspace: Color | `workspace.color` | shipped | Shift+4 | `{}` |
| Workspace: Fairlight | `workspace.audio` | shipped | Shift+5 | `{}` |
| Workspace: Effects | `workspace.effects` | shipped | Shift+6 | `{}` |
| Workspace: Animation | `workspace.keyframes` | shipped | Shift+7 | `{}` |
| Workspace: Deliver | `workspace.deliver` | shipped | Shift+8 | `{}` |
| Workspace: Library | `workspace.library` | planned |  | `{}` |
| Workspace: AI Assistant | `workspace.ai` | planned |  | `{}` |
| Workspace: Export | `workspace.export` | planned |  | `{}` |

## Help

| Command | ID | Status | Shortcut | Params |
|---|---|---|---|---|
| Keyboard Shortcuts | `help.shortcuts` | shipped | Ctrl+/ | `{}` |
| Copy System Diagnostics | `help.diagnostics` | shipped |  | `{}` |
| Report Issue / Support | `help.support` | shipped |  | `{}` |
| About Easy Edits Pro | `help.about` | shipped |  | `{}` |
