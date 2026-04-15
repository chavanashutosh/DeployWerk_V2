# Example Terraform layout for a future `deploywerk` provider (Phase 8).
# This repository does not ship the provider binary yet; keep definitions as design guidance.

# terraform {
#   required_providers {
#     deploywerk = {
#       source  = "deploywerk/deploywerk"
#       version = "~> 0.1"
#     }
#   }
# }
#
# provider "deploywerk" {
#   base_url = "https://deploywerk.example.com"
#   token    = var.deploywerk_token
# }
#
# resource "deploywerk_team" "main" {
#   name = "Platform"
# }
#
# resource "deploywerk_project" "api" {
#   team_id = deploywerk_team.main.id
#   name    = "API"
# }
