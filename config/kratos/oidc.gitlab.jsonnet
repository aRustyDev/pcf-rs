local claims = {
  email_verified: false,
} + std.extVar('claims');

{
  identity: {
    traits: {
      // GitLab provides email
      [if 'email' in claims && claims.email_verified then 'email' else null]: claims.email,

      // Map GitLab user info to name
      name: {
        first: if 'given_name' in claims then claims.given_name else if 'name' in claims then std.split(claims.name, ' ')[0] else 'Unknown',
        last: if 'family_name' in claims then claims.family_name else if 'name' in claims && std.length(std.split(claims.name, ' ')) > 1 then std.join(' ', std.slice(std.split(claims.name, ' '), 1, std.length(std.split(claims.name, ' ')), 1)) else 'User',
      },

      // Store GitLab-specific data
      gitlab: {
        username: if 'preferred_username' in claims then claims.preferred_username else null,
        id: if 'sub' in claims then claims.sub else null,
        avatar_url: if 'picture' in claims then claims.picture else null,
      },
    },
  },
}
