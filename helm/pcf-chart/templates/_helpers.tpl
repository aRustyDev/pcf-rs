{{/*
Expand the name of the chart.
*/}}
{{- define "pcf-chart.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "pcf-chart.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "pcf-chart.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "pcf-chart.labels" -}}
helm.sh/chart: {{ include "pcf-chart.chart" . }}
{{ include "pcf-chart.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "pcf-chart.selectorLabels" -}}
app.kubernetes.io/name: {{ include "pcf-chart.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "pcf-chart.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "pcf-chart.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Linkerd annotations
*/}}
{{- define "pcf-chart.linkerdAnnotations" -}}
{{- if .Values.global.linkerd.enabled }}
linkerd.io/inject: {{ .Values.global.linkerd.inject | quote }}
{{- end }}
{{- end }}

{{/*
Get the namespace
*/}}
{{- define "pcf-chart.namespace" -}}
{{- if .Values.namespace.create }}
{{- .Values.namespace.name }}
{{- else }}
{{- .Release.Namespace }}
{{- end }}
{{- end }}