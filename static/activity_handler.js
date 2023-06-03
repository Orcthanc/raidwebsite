// Assuming the server returns an array of updated activities
// with their id, completed status, and available status
async function updateActivityOnServer(characterId, activityId, completed) {
    const formData = new FormData();
    formData.append("character_id", characterId);
    formData.append("activity_id", activityId);
    formData.append("completed", completed);

    const data = new URLSearchParams();
    for (const pair of formData) {
        data.append(pair[0], pair[1]);
    }
  
    const response = await fetch("update_activity", {
      method: "POST",
      body: data,
    });
  
    if (!response.ok) {
      throw new Error(`Failed to update activity: ${response.statusText}`);
    }
  
    return response.json(); // Return the updated activities
  }
  
  function updateActivityState(activityBox, completed, available) {
    activityBox.classList.remove("completed", "not-completed", "unavailable");
  
    if (completed) {
      activityBox.classList.add("completed");
    } else if (available) {
      activityBox.classList.add("not-completed");
    } else {
      activityBox.classList.add("unavailable");
    }
  }
  
  async function toggleActivity(event) {
    const activityBox = event.target;
    const characterId = activityBox.dataset.characterId;
    const activityId = activityBox.dataset.activityId;
    const currentState = activityBox.classList.contains("completed") ? "completed" : activityBox.classList.contains("not-completed") ? "not-completed" : "unavailable";
  
    if (currentState === "unavailable") {
      // Do not update unavailable activities
      return;
    }
  
    const nextState = currentState === "completed" ? "not-completed" : "completed";
  
    try {
      const updatedActivities = await updateActivityOnServer(characterId, activityId, nextState === "completed");
  
      // Update the activity states in the UI
      updatedActivities.forEach(({ id, completed, available }) => {
        const updatedActivityBox = document.querySelector(`.activity-box[data-character-id="${characterId}"][data-activity-id="${id}"]`);
        updateActivityState(updatedActivityBox, completed, available);
      });
    } catch (error) {
      console.error("Error updating activity:", error);
    }
  }