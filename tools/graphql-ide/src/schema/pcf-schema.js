const pcfSchema = `# PCF GraphQL Schema

## Base Types & Enums

enum TaskStatus {
  PLANNED
  IN_PROGRESS
  BLOCKED
  COMPLETED
  CANCELLED
}

enum TaskPriority {
  CRITICAL
  HIGH
  MEDIUM
  LOW
}

enum FindingSeverity {
  CRITICAL
  HIGH
  MEDIUM
  LOW
  INFO
}

enum CredentialType {
  PASSWORD
  SSH_KEY
  API_TOKEN
  CERTIFICATE
  GPG_KEY
  OAUTH_TOKEN
  OTHER
}

enum AccountType {
  PERSONAL
  TARGET
  TEAM
}

## Common Interfaces

interface Node {
  id: ID!
  createdAt: DateTime!
  updatedAt: DateTime!
}

interface Auditable {
  createdBy: Person!
  updatedBy: Person!
}

## Core Domain Types

type Organization implements Node {
  id: ID!
  name: String!
  description: String
  teams: [Team!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Team implements Node {
  id: ID!
  name: String!
  description: String
  members: [Person!]!
  projects: [Project!]!
  accounts: [TeamAccount!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Project implements Node & Auditable {
  id: ID!
  name: String!
  description: String
  team: Team!
  tasks: [Task!]!
  findings: [Finding!]!
  reports: [Report!]!
  startDate: DateTime
  endDate: DateTime
  status: String!
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type Person implements Node {
  id: ID!
  name: String!
  email: String!
  teams: [Team!]!
  personalAccounts: [PersonalAccount!]!
  assignedTasks: [Task!]!
  createdFindings: [Finding!]!
  toolsUsed: [Tool!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

## Network Infrastructure Types

type Network implements Node {
  id: ID!
  name: String!
  cidr: String!
  description: String
  hosts: [Host!]!
  vlans: [VLAN!]!
  services: [Service!]!
  project: Project!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type VLAN implements Node {
  id: ID!
  vlanId: Int!
  name: String
  description: String
  network: Network!
  hosts: [Host!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Host implements Node {
  id: ID!
  hostname: String
  ipAddresses: [String!]!
  macAddresses: [String!]
  osInfo: OSInfo
  networks: [Network!]!
  vlans: [VLAN!]!
  services: [Service!]!
  accounts: [TargetAccount!]!
  credentials: [Credential!]!
  files: [File!]!
  tasks: [Task!]!
  findings: [Finding!]!
  project: Project!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Service implements Node {
  id: ID!
  name: String!
  port: Int
  protocol: String
  version: String
  hosts: [Host!]!
  networks: [Network!]!
  vlans: [VLAN!]!
  accounts: [Account!]!
  credentials: [Credential!]!
  findings: [Finding!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type OSInfo {
  name: String!
  version: String
  architecture: String
  kernel: String
  additionalInfo: JSON
}

## Account Types

interface Account {
  id: ID!
  identifier: Identifier!
  credentials: [Credential!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type PersonalAccount implements Node & Account {
  id: ID!
  identifier: Identifier!
  owner: Person!
  platform: String!
  credentials: [Credential!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type TargetAccount implements Node & Account {
  id: ID!
  identifier: Identifier!
  targetPerson: Person
  targetHost: Host
  targetService: Service
  credentials: [Credential!]!
  privileges: [String!]
  createdAt: DateTime!
  updatedAt: DateTime!
}

type TeamAccount implements Node & Account {
  id: ID!
  identifier: Identifier!
  team: Team!
  purpose: String!
  credentials: [Credential!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Identifier implements Node {
  id: ID!
  value: String!
  type: String!
  accounts: [Account!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Credential implements Node {
  id: ID!
  type: CredentialType!
  name: String
  encryptedValue: String!
  accounts: [Account!]!
  hosts: [Host!]!
  services: [Service!]!
  lastUsed: DateTime
  expiresAt: DateTime
  createdAt: DateTime!
  updatedAt: DateTime!
}

## Task Management Types

type Task implements Node & Auditable {
  id: ID!
  title: String!
  description: String!
  status: TaskStatus!
  priority: TaskPriority!
  project: Project!
  assignees: [Person!]!
  parentTask: Task
  childTasks: [Task!]!
  relatedHosts: [Host!]!
  relatedServices: [Service!]!
  findings: [Finding!]!
  notes: [Note!]!
  comments: [Comment!]!
  actions: [Action!]!
  dueDate: DateTime
  completedAt: DateTime
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type Finding implements Node & Auditable {
  id: ID!
  title: String!
  description: String!
  severity: FindingSeverity!
  task: Task!
  affectedHosts: [Host!]
  affectedNetworks: [Network!]
  affectedServices: [Service!]
  indicators: [Indicator!]!
  proofOfConcepts: [PoC!]!
  remediationSteps: String
  references: [String!]
  verified: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

## Supporting Types

type Tool implements Node {
  id: ID!
  name: String!
  version: String
  description: String
  users: [Person!]!
  actions: [Action!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Action implements Node & Auditable {
  id: ID!
  name: String!
  command: String
  output: String
  tool: Tool!
  task: Task!
  executedBy: Person!
  executedAt: DateTime!
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type Pattern implements Node {
  id: ID!
  name: String!
  regex: String!
  description: String
  category: String
  createdAt: DateTime!
  updatedAt: DateTime!
}

type PoC implements Node {
  id: ID!
  name: String!
  description: String
  type: String!
  externalUrl: String
  code: String
  findings: [Finding!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Indicator implements Node {
  id: ID!
  name: String!
  type: String!
  value: String!
  confidence: Float
  findings: [Finding!]!
  filePath: String
  scriptUrl: String
  evidence: JSON
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Note implements Node & Auditable {
  id: ID!
  content: String!
  task: Task
  project: Project
  attachments: [File!]
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type Comment implements Node & Auditable {
  id: ID!
  content: String!
  task: Task!
  parentComment: Comment
  replies: [Comment!]!
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type Report implements Node & Auditable {
  id: ID!
  name: String!
  template: String!
  project: Project!
  includedFindings: [Finding!]!
  includedTasks: [Task!]!
  generatedContent: String
  generatedAt: DateTime
  createdAt: DateTime!
  updatedAt: DateTime!
  createdBy: Person!
  updatedBy: Person!
}

type File implements Node {
  id: ID!
  filename: String!
  mimeType: String!
  size: Int!
  storageKey: String!
  host: Host
  note: Note
  createdAt: DateTime!
  updatedAt: DateTime!
}

## Pagination Types

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

type TaskConnection {
  edges: [TaskEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type TaskEdge {
  cursor: String!
  node: Task!
}

## Input Types

input ProjectCreateInput {
  name: String!
  description: String
  teamId: ID!
  startDate: DateTime
  endDate: DateTime
}

input TaskCreateInput {
  title: String!
  description: String!
  status: TaskStatus
  priority: TaskPriority!
  projectId: ID!
  assigneeIds: [ID!]
  parentTaskId: ID
  relatedHostIds: [ID!]
  relatedServiceIds: [ID!]
  dueDate: DateTime
}

input FindingCreateInput {
  title: String!
  description: String!
  severity: FindingSeverity!
  taskId: ID!
  affectedHostIds: [ID!]
  affectedNetworkIds: [ID!]
  affectedServiceIds: [ID!]
  remediationSteps: String
  references: [String!]
}

input HostCreateInput {
  hostname: String
  ipAddresses: [String!]!
  macAddresses: [String!]
  networkIds: [ID!]!
  projectId: ID!
}

input AccountFilter {
  type: AccountType
  ownerId: ID
  teamId: ID
}

input TaskFilter {
  status: TaskStatus
  priority: TaskPriority
  assigneeId: ID
  projectId: ID
  hasFindings: Boolean
}

input FindingFilter {
  severity: FindingSeverity
  verified: Boolean
  projectId: ID
  hostId: ID
  networkId: ID
}

## Root Query Type

type Query {
  # Organization & Team Queries
  organization(id: ID!): Organization
  organizations(first: Int, after: String): OrganizationConnection!
  
  team(id: ID!): Team
  teams(organizationId: ID, first: Int, after: String): TeamConnection!
  
  # Project Queries
  project(id: ID!): Project
  projects(teamId: ID, status: String, first: Int, after: String): ProjectConnection!
  
  # Person Queries
  person(id: ID!): Person
  people(teamId: ID, first: Int, after: String): PersonConnection!
  currentUser: Person!
  
  # Infrastructure Queries
  network(id: ID!): Network
  networks(projectId: ID!, first: Int, after: String): NetworkConnection!
  
  host(id: ID!): Host
  hosts(
    projectId: ID!
    networkId: ID
    hasFindings: Boolean
    first: Int
    after: String
  ): HostConnection!
  
  service(id: ID!): Service
  services(
    hostId: ID
    networkId: ID
    port: Int
    first: Int
    after: String
  ): ServiceConnection!
  
  # Account Queries
  account(id: ID!): Account
  accounts(filter: AccountFilter, first: Int, after: String): AccountConnection!
  
  # Task Management Queries
  task(id: ID!): Task
  tasks(
    filter: TaskFilter
    orderBy: String
    first: Int
    after: String
  ): TaskConnection!
  
  finding(id: ID!): Finding
  findings(
    filter: FindingFilter
    orderBy: String
    first: Int
    after: String
  ): FindingConnection!
  
  # Search Queries
  searchHosts(query: String!, projectId: ID!): [Host!]!
  searchServices(query: String!, projectId: ID!): [Service!]!
  searchFindings(query: String!, projectId: ID!): [Finding!]!
  
  # Reporting Queries
  report(id: ID!): Report
  reports(projectId: ID!, first: Int, after: String): ReportConnection!
  
  # Pattern Matching
  patterns(category: String): [Pattern!]!
  matchPattern(patternId: ID!, text: String!): Boolean!
}

## Root Mutation Type

type Mutation {
  # Organization & Team Mutations
  createOrganization(name: String!, description: String): Organization!
  updateOrganization(id: ID!, name: String, description: String): Organization!
  deleteOrganization(id: ID!): Boolean!
  
  createTeam(name: String!, description: String, organizationId: ID): Team!
  addTeamMember(teamId: ID!, personId: ID!): Team!
  removeTeamMember(teamId: ID!, personId: ID!): Team!
  
  # Project Mutations
  createProject(input: ProjectCreateInput!): Project!
  updateProject(id: ID!, name: String, description: String, status: String): Project!
  archiveProject(id: ID!): Project!
  
  # Infrastructure Mutations
  createHost(input: HostCreateInput!): Host!
  updateHost(id: ID!, hostname: String, ipAddresses: [String!]): Host!
  deleteHost(id: ID!): Boolean!
  
  createService(
    name: String!
    port: Int
    protocol: String
    hostIds: [ID!]!
  ): Service!
  updateService(id: ID!, name: String, version: String): Service!
  
  createNetwork(
    name: String!
    cidr: String!
    projectId: ID!
    description: String
  ): Network!
  
  # Account & Credential Mutations
  createPersonalAccount(
    identifierId: ID!
    ownerId: ID!
    platform: String!
  ): PersonalAccount!
  
  createTargetAccount(
    identifierId: ID!
    targetPersonId: ID
    targetHostId: ID
    targetServiceId: ID
  ): TargetAccount!
  
  createCredential(
    type: CredentialType!
    name: String
    accountIds: [ID!]!
  ): Credential!
  
  associateCredential(credentialId: ID!, accountId: ID!): Account!
  
  # Task Management Mutations
  createTask(input: TaskCreateInput!): Task!
  updateTask(
    id: ID!
    title: String
    description: String
    status: TaskStatus
    priority: TaskPriority
  ): Task!
  assignTask(taskId: ID!, assigneeIds: [ID!]!): Task!
  completeTask(id: ID!): Task!
  
  # Finding Mutations
  createFinding(input: FindingCreateInput!): Finding!
  updateFinding(
    id: ID!
    title: String
    description: String
    severity: FindingSeverity
    verified: Boolean
  ): Finding!
  verifyFinding(id: ID!): Finding!
  
  addIndicator(
    findingId: ID!
    name: String!
    type: String!
    value: String!
    confidence: Float
  ): Indicator!
  
  addProofOfConcept(
    findingId: ID!
    name: String!
    type: String!
    externalUrl: String
    code: String
  ): PoC!
  
  # Collaboration Mutations
  addComment(taskId: ID!, content: String!, parentCommentId: ID): Comment!
  updateComment(id: ID!, content: String!): Comment!
  deleteComment(id: ID!): Boolean!
  
  addNote(
    content: String!
    taskId: ID
    projectId: ID
  ): Note!
  
  # Reporting Mutations
  generateReport(
    projectId: ID!
    templateId: ID!
    findingIds: [ID!]
    taskIds: [ID!]
  ): Report!
}

## Root Subscription Type

type Subscription {
  # Real-time Task Updates
  taskUpdated(projectId: ID!): Task!
  taskCreated(projectId: ID!): Task!
  taskAssigned(personId: ID!): Task!
  
  # Real-time Finding Updates
  findingCreated(projectId: ID!): Finding!
  findingVerified(projectId: ID!): Finding!
  
  # Host Discovery
  hostDiscovered(projectId: ID!): Host!
  serviceDiscovered(projectId: ID!): Service!
  
  # Collaboration
  commentAdded(taskId: ID!): Comment!
  
  # External Agent Updates
  agentUpdate(agentId: ID!): AgentUpdate!
}

type AgentUpdate {
  agentId: ID!
  type: String!
  data: JSON!
  timestamp: DateTime!
}

# Connection types for all entities would be defined here...
type OrganizationConnection {
  edges: [OrganizationEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type OrganizationEdge {
  cursor: String!
  node: Organization!
}

type TeamConnection {
  edges: [TeamEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type TeamEdge {
  cursor: String!
  node: Team!
}

type ProjectConnection {
  edges: [ProjectEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type ProjectEdge {
  cursor: String!
  node: Project!
}

type PersonConnection {
  edges: [PersonEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type PersonEdge {
  cursor: String!
  node: Person!
}

type NetworkConnection {
  edges: [NetworkEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type NetworkEdge {
  cursor: String!
  node: Network!
}

type HostConnection {
  edges: [HostEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type HostEdge {
  cursor: String!
  node: Host!
}

type ServiceConnection {
  edges: [ServiceEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type ServiceEdge {
  cursor: String!
  node: Service!
}

type AccountConnection {
  edges: [AccountEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type AccountEdge {
  cursor: String!
  node: Account!
}

type FindingConnection {
  edges: [FindingEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type FindingEdge {
  cursor: String!
  node: Finding!
}

type ReportConnection {
  edges: [ReportEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type ReportEdge {
  cursor: String!
  node: Report!
}

# Custom Scalars
scalar DateTime
scalar JSON

schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}`;

export default pcfSchema;
