function(ctx) {
  identity_id: ctx.identity.id,
  email: ctx.identity.traits.email,
  name: ctx.identity.traits.name,
  schema_id: ctx.identity.schema_id,
  created_at: ctx.identity.created_at
}
