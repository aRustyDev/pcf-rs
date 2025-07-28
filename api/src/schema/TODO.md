# TODO

## Person

Description of a person/user/admin/etc
Are member of a `Organinization`, and can be guests of another `Organinization`
Are member of a `Project`, and can be guests of another `Project`
Are member of a `Team`, and can be guests of another `Team`

## Account

Accounts can be used by a `Person` or a `Service`
Belong to an `Organinization`
Can login to a `Service`
Can login to a `Host`

## Service

Description of a service/application. It is a machine that has a process that runs it.
Run on a `Host` or `Platform`

## Comment

Comments are arbitrary newline delimited text.
have an owner (`Person`)
can be associated with a `Team` or `Organinization`
can be "About" any entity OR "Reply" to another `Comment`.
can be "Root" comment or "Child" comment.

## Organinization

Organinizations are a root collection of `Team`s, `Project`s, `Person`s, & `Account`s.

## Platform

Platforms are a hosting platforms for services

## Host

Hosts are individual machines
run/host `Service`s

## File

Files are individual `File`s

## Directory

Directories are hierarchical collections of `File`s sub `Directory`s

## Secret

Secrets are a super type for sensitive information.
- Examples: Passwords, API keys, Private Keys, Bearer Tokens, JWT, OAuth Tokens, GPG Keys, SSH Keys, Secret Text, Recovery Codes, Certificate
- Categories: Personal, Target, Team, Organization, Project
Have a purpose (e.g., authentication, encryption, authorization)
Have MACD dates (Modified, accessed, Created, Deleted)
Can have expiration date
Can Authenticate `Service`
Used for `Domain`|`Service`
Used By `Account`

## Hash

Hashes are hashed values of sensitive information.

## Network

Networks are a collection of `CIDR`

## CIDR

CIDR is a range of `IP` addresses

## IP

IP is a logical network address

## MAC

MAC is a physical network address

## Finding



## Group

## Pattern

## Tool

## PoC

## Job

## Identifier

## OSInfo

## Action

## ASN

## VLAN

## Task
