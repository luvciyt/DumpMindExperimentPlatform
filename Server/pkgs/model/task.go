package model

import (
	"time"
)

type TaskType string

const (
	TaskTypeGetVmcore  TaskType = "get-vmcore"
	TaskTypePatchApply TaskType = "patch-apply"
)

type TaskStatus string

const (
	StatusPending TaskStatus = "pending"
	StatusRunning TaskStatus = "running"
	StatusSuccess TaskStatus = "success"
	StatusFailed  TaskStatus = "failed"
)

type Task struct {
	ID     string     `json:"id" gorm:"type:char(36);primaryKey"` // UUID 字符串
	Type   TaskType   `json:"type" gorm:"type:varchar(32)"`
	Status TaskStatus `json:"status" gorm:"type:varchar(32)"`
	//Payload      CrashReport   `json:"payload" gorm:"type:json"`
	WorkerID     string     `json:"worker_id" gorm:"type:varchar(64);index"`
	Result       string     `json:"result" gorm:"type:text"`
	ArtifactPath string     `json:"artifact_path" gorm:"type:text"`
	ArtifactName string     `json:"artifact_name" gorm:"type:text"`
	CreatedAt    time.Time  `json:"created_at"`
	StartedAt    *time.Time `json:"started_at"`
	FinishedAt   *time.Time `json:"finished_at"`
}
